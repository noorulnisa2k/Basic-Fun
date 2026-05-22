mod order_structure;
mod prism_structure;

use anyhow::{Result, anyhow};

use bb8::Pool;
use clap::Parser;
use dotenv::dotenv;
use order_structure::{BatchNumbers, DocumentLine, Orders, Token, Tracking};
// use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE, COOKIE};
// use reqwest::{Client, Method};
use roxmltree_to_serde::{Config, NullValue, xml_str_to_json};
// use rust_decimal::Decimal;
// use serde_json::json;
use futures::future::join_all;
use prism_structure::Output;
use reqwest_middleware::reqwest::header::{CONTENT_TYPE, COOKIE, HeaderMap, HeaderValue};
use reqwest_middleware::reqwest::{Client, Method, Response, StatusCode};
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::{RetryTransientMiddleware, Retryable, policies::ExponentialBackoff};
use roxmltree::Document;
use serde_json::{Value, json};
use core::f64;
use std::borrow::Cow;
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tiberius::numeric::{Numeric, Decimal};
use tiberius::{Row, ToSql};
use tokio::sync::{Mutex, Semaphore};
use tokio::time::Instant;
use tracing::{debug, error, info, warn};
use tracing_subscriber::EnvFilter;

use crate::prism_structure::Agency;
use serde::Serialize;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    files_pickup_path: PathBuf,

    #[arg(short, long)]
    process_id: String,

    #[arg(long)]
    process_data_path: PathBuf,

    #[arg(long)]
    prism_path: PathBuf,
}

const BASE_URL: &str = "https://BFAZWSAP01.corp.basicfun.com:50000";
// const BASE_URL: &str = "https://10.1.0.7:50000";
const THREADS: usize = 8;

pub struct FullRetryableStrategy;
impl reqwest_retry::RetryableStrategy for FullRetryableStrategy {
    fn handle(
        &self,
        res: &Result<reqwest_middleware::reqwest::Response, reqwest_middleware::Error>,
    ) -> Option<Retryable> {
        match res {
            Ok(success) => default_on_request_success(success),
            Err(error) => default_on_request_failure(error),
        }
    }
}

pub fn default_on_request_success(
    success: &reqwest_middleware::reqwest::Response,
) -> Option<Retryable> {
    let status = success.status();
    if status.is_server_error() {
        Some(Retryable::Transient)
    } else if status.is_client_error()
        && status != StatusCode::BAD_REQUEST
        && status != StatusCode::REQUEST_TIMEOUT
        && status != StatusCode::TOO_MANY_REQUESTS
        && status != StatusCode::INTERNAL_SERVER_ERROR
        && status != StatusCode::BAD_GATEWAY
        && status != StatusCode::NOT_FOUND
    {
        info!("Fatal Retry Status Code: {status}");
        Some(Retryable::Fatal)
    } else if status.is_success() {
        None
    } else if status == StatusCode::REQUEST_TIMEOUT
        || status == StatusCode::BAD_REQUEST
        || status == StatusCode::TOO_MANY_REQUESTS
        || status == StatusCode::INTERNAL_SERVER_ERROR
        || status == StatusCode::BAD_GATEWAY
        || status == StatusCode::NOT_FOUND
    {
        info!("Retry Status Code: {status}");
        Some(Retryable::Transient)
    } else {
        Some(Retryable::Fatal)
    }
}

pub fn default_on_request_failure(error: &reqwest_middleware::Error) -> Option<Retryable> {
    match error {
        // If something fails in the middleware we're screwed.
        reqwest_middleware::Error::Middleware(_) => Some(Retryable::Fatal),
        reqwest_middleware::Error::Reqwest(error) => {
            #[cfg(not(target_arch = "wasm32"))]
            let is_connect = error.is_connect();
            #[cfg(target_arch = "wasm32")]
            let is_connect = false;
            if error.is_timeout() || is_connect {
                Some(Retryable::Transient)
            } else if error.is_body()
                || error.is_decode()
                || error.is_builder()
                || error.is_redirect()
            {
                Some(Retryable::Fatal)
            } else if error.is_request() {
                // It seems that hyper::Error(IncompleteMessage) is not correctly handled by reqwest.
                // Here we check if the Reqwest error was originated by hyper and map it consistently.
                #[cfg(not(target_arch = "wasm32"))]
                if let Some(hyper_error) = get_source_error_type::<hyper::Error>(&error) {
                    // The hyper::Error(IncompleteMessage) is raised if the HTTP response is well formatted but does not contain all the bytes.
                    // This can happen when the server has started sending back the response but the connection is cut halfway through.
                    // We can safely retry the call, hence marking this error as [`Retryable::Transient`].
                    // Instead hyper::Error(Canceled) is raised when the connection is
                    // gracefully closed on the server side.
                    if hyper_error.is_incomplete_message() || hyper_error.is_canceled() {
                        Some(Retryable::Transient)

                    // Try and downcast the hyper error to io::Error if that is the
                    // underlying error, and try and classify it.
                    } else if let Some(io_error) =
                        get_source_error_type::<std::io::Error>(hyper_error)
                    {
                        Some(classify_io_error(io_error))
                    } else {
                        Some(Retryable::Fatal)
                    }
                } else {
                    Some(Retryable::Fatal)
                }
                #[cfg(target_arch = "wasm32")]
                Some(Retryable::Fatal)
            } else {
                // We omit checking if error.is_status() since we check that already.
                // However, if Response::error_for_status is used the status will still
                // remain in the response object.
                None
            }
        }
    }
}

fn classify_io_error(error: &std::io::Error) -> Retryable {
    match error.kind() {
        std::io::ErrorKind::ConnectionReset | std::io::ErrorKind::ConnectionAborted => {
            Retryable::Transient
        }
        _ => Retryable::Fatal,
    }
}

/// Downcasts the given err source into T.
fn get_source_error_type<T: std::error::Error + 'static>(
    err: &dyn std::error::Error,
) -> Option<&T> {
    let mut source = err.source();

    while let Some(err) = source {
        if let Some(err) = err.downcast_ref::<T>() {
            return Some(err);
        }

        source = err.source();
    }
    None
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_ansi(false)
        .with_env_filter(EnvFilter::new("info,tiberius=warn"))
        .init();
    dotenv().ok();

    let args = Args::parse();
    let files_pickup_path = Arc::new(args.files_pickup_path);
    let process_data_path = args.process_data_path;
    let prism_path = Arc::new(args.prism_path);

    // let db_pool = Arc::new(setup_db_pool("DB_CONN").await?);
    let sap_pool = Arc::new(setup_db_pool("SAP_DB_CONN").await?);

    let (company, username, password) = (
        env::var("Company_DB").expect("Company_DB must be set"),
        env::var("User_Name").expect("User_Name must be set"),
        env::var("Password").expect("Password must be set"),
    );

    let base_client = match Client::builder()
        .danger_accept_invalid_certs(true)
        .timeout(Duration::from_secs(60))
        .build()
    {
        Ok(client) => client,
        Err(err) => {
            error!("Failed to build reqwest client: {err}");
            return Err("Failed to build reqwest HTTP client".into());
        }
    };

    // Configure retry policy
    let retry_policy =
        ExponentialBackoff::builder()
        .retry_bounds(Duration::from_millis(100), Duration::from_secs(30))
        .build_with_total_retry_duration(Duration::from_secs(2 * 60));

    let ret_s =
        RetryTransientMiddleware::new_with_policy_and_strategy(retry_policy, FullRetryableStrategy);

    // Wrap with retry middleware
    let client = ClientBuilder::new(base_client).with(ret_s).build();

    // Wrap with retry middleware
    /*
    let client = ClientBuilder::new(base_client)
        .with(RetryTransientMiddleware::new_with_policy(retry_policy))
        .build();
    */
    let client = Arc::new(client);

    let start = Instant::now();
    let token = get_token(&company, &username, &password, &client).await?;
    let session_id = Arc::new(["B1SESSION=", token.session_id.as_str()].concat());

    let file_hashmap = match fetch_prism_data(process_data_path) {
        Ok(file_hashmap) => Arc::new(file_hashmap),
        Err(err) => return Err(format!("Failed to generate ERP file HashMap: {err}").into()),
    };

    debug!("ERP HashMap: {file_hashmap:?}");

    match process_files(
        &files_pickup_path,
        &sap_pool,
        &session_id,
        &client,
        &file_hashmap,
        &prism_path,
    )
    .await
    {
        Ok(_) => {}
        Err(e) => return Err(format!("IB 945 Processing Errors {e}").into()),
    };

    info!("Processing Time {:?}", start.elapsed());
    Ok(())
}

async fn setup_db_pool(env_var: &str) -> Result<Pool<bb8_tiberius::ConnectionManager>> {
    let conn_str = env::var(env_var)?;
    let mgr = bb8_tiberius::ConnectionManager::build(conn_str.as_str())?;
    let pool = bb8::Pool::builder()
        .max_size(THREADS.try_into().unwrap())
        .build(mgr)
        .await?;
    Ok(pool)
}

fn fetch_prism_data(
    process_data_path: PathBuf,
) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
    let mut file_hashmap: HashMap<String, String> = HashMap::new();
    for file in std::fs::read_dir(&process_data_path)? {
        let file = file?;
        let path = file.path();

        if path.is_file() {
            let process_input = match std::fs::read_to_string(&path) {
                Ok(process_input) => process_input,
                Err(err) => return Err(format!("Failed to Read Process Data: {err}").into()),
            };

            let process_data: Output = match quick_xml::de::from_str(&process_input) {
                Ok(process_data) => process_data,
                Err(err) => {
                    error!(
                        "Failed to Parse Process Data archive file: {err}\n {}",
                        process_input.as_str()
                    );
                    return Err("Failed to Parse Process Data".into());
                }
            };

            if let Some(erp_file_name) = process_data.erp_file_name {
                file_hashmap.insert(
                    Path::new(&erp_file_name.into_owned())
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .to_string(),
                    path.to_string_lossy().to_string(),
                );
            }

            if let Some(edi_file_name) = process_data.edi_file_name {
                file_hashmap.insert(
                    Path::new(&edi_file_name.into_owned())
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .to_string(),
                    path.to_string_lossy().to_string(),
                );
            }
        }
    }

    Ok(file_hashmap)
}

async fn process_files(
    path: &Path,
    sap_pool: &Arc<Pool<bb8_tiberius::ConnectionManager>>,
    session_id: &Arc<String>,
    client: &Arc<ClientWithMiddleware>,
    file_hashmap: &Arc<HashMap<String, String>>,
    prism_path: &Arc<PathBuf>,
) -> Result<(), anyhow::Error> {
    let semaphore = Arc::new(Semaphore::new(THREADS));
    let mut handles = Vec::new();
    let global_lock = Arc::new(Mutex::new(()));

    for entry in match std::fs::read_dir(path) {
        Ok(entries) => entries,
        Err(e) => {
            error!("Failed to read dir {}: {e}", path.display());
            return Err(anyhow!("Failed to read dir {}: {e}", path.display()));
        }
    } {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                error!("Failed to read entry: {e}");
                continue;
            }
        };

        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let sap_pool = Arc::clone(sap_pool);
        let client = Arc::clone(client);
        let session_id = Arc::clone(session_id);
        let file_hashmap = Arc::clone(file_hashmap);
        let prism_path = Arc::clone(prism_path);
        let lock = Arc::clone(&global_lock);

        handles.push(tokio::spawn(async move {
            let _permit = permit;
            let erp_file_path = path.file_name().unwrap().to_string_lossy().to_string();
            info!("ERP File Path to Pull: {erp_file_path}");
            let prism_file = match file_hashmap.get(&erp_file_path) {
                Some(prism_file) => prism_file,
                None => {
                    error!("No Prism Process Data file associated with {erp_file_path}");
                    ""
                    /*
                    return Err(anyhow!(
                        "No Prism Process Data file associated with {erp_file_path}"
                    ));
                    */
                }
            };

            match process_file(&path, &sap_pool, &session_id, &client, lock).await {
                Ok(_) => {
                    info!("Processed file: {}", path.display());

                    if !prism_file.is_empty() {
                        let mut prism_file_path = PathBuf::from(prism_path.as_ref());
                        prism_file_path.push(Path::new(prism_file).file_name().unwrap());
                        match tokio::fs::rename(prism_file, &prism_file_path).await {
                        Ok(_) => {
                            info!("Moved {prism_file} to {}", prism_file_path.display());
                        }
                        Err(err) => {
                            error!(
                                "Failed to move {prism_file} to {}: {err}",
                                &prism_file_path.display()
                            );
                            return Err(anyhow!(
                                "Failed to move {prism_file} to {}: {err}",
                                prism_file_path.display()
                                ));
                            }
                        }
                    }
                    
                    match tokio::fs::remove_file(&path).await {
                        Ok(_) => {
                            info!("Removed {}", path.display());
                        }
                        Err(err) => {
                            error!("Failed to remove 945 XML {}: {err}", path.display());
                            return Err(anyhow!(
                                "Failed to remove 945 XML {}: {err}",
                                path.display()
                            ));
                        }
                    };
                    Ok(())
                }
                Err(e) => {
                    if !prism_file.is_empty() {
                        let mut prism_file_path = PathBuf::from(prism_path.as_ref());
                        prism_file_path.push(Path::new(prism_file).file_name().unwrap());
                        info!("Updating Process Data file {prism_file} with error {e}",);
                        let process_input = match std::fs::read_to_string(prism_file) {
                            Ok(process_input) => process_input,
                            Err(err) => {
                                return Err(anyhow!("Failed to Read Process Data: {err}"));
                            }
                        };

                        let mut process_data: Output = match quick_xml::de::from_str(&process_input) {
                            Ok(process_data) => process_data,
                            Err(err) => {
                                error!(
                                    "Failed to Parse Process Data archive file: {err}\n {}",
                                    process_input.as_str()
                                );
                                return Err(anyhow!("Failed to Parse Process Data"));
                            }
                        };
                        info!("Process Data to Modify {process_data:?}");
                        process_data.error_type = Some(Cow::Borrowed("ERP"));
                        process_data.agency = Agency::ERROR;
                        process_data.error_description = Some(Cow::from(e.to_string()));
                        process_data.status = Cow::Borrowed("ERROR");
                        process_data.plant_name = Some(Cow::Borrowed("BasicFun"));

                        // Set Reference Number
                        let erp_data =
                            std::fs::read_to_string(&path).expect("Failed to read 944 XML file");
                        let doc = Document::parse(&erp_data)?;
                        if let Some(doc_num) =
                            doc.descendants().find(|n| n.tag_name().name() == "DocNum")
                            && let Some(text) = doc_num.text()
                        {
                            info!("DocNum: {}", text);
                            process_data.reference = Some(Cow::Borrowed(text));
                        }

                        let mut xml_string = String::new();
                        let ser = quick_xml::se::Serializer::with_root(&mut xml_string, Some("Output"))
                            .unwrap();
                        process_data.serialize(ser).unwrap();
                        match tokio::fs::write(&prism_file_path, &xml_string).await {
                            Ok(_) => {
                                info!("Wrote {} to {}", &xml_string, &prism_file_path.display())
                            }
                            Err(err) => {
                                error!(
                                    "Failed to write error process data to {}: {err}",
                                    &path.display()
                                );
                            }
                        };
                    }
                    

                    match tokio::fs::remove_file(&path).await {
                        Ok(_) => {}
                        Err(err) => {
                            error!("Failed to remove 945 XML {}: {err}", path.display());
                            return Err(anyhow!(
                                "Failed to remove 945 XML {}: {err}",
                                path.display()
                            ));
                        }
                    };
                    // error!("Failed to process {}: {e}", path.display());
                    Err(anyhow!("Failed to process {}: {e}", path.display()))
                }
            }
        }));
    }

    let results = join_all(handles).await;
    debug!("Results: {results:?}");

    let mut error_count = 0;

    for result in results {
        match result {
            Ok(Ok(())) => {
                // Task completed successfully
            }
            Ok(Err(e)) => {
                error!("{e}");
                error_count += 1;
            }
            Err(e) => {
                error!("Task panicked: {e}");
                error_count += 1;
            }
        }
    }

    if error_count > 0 {
        error!("{error_count} tasks failed");
        std::process::exit(1);
    }
    Ok(())
}

async fn process_file(
    path: &Path,
    sap_pool: &Arc<Pool<bb8_tiberius::ConnectionManager>>,
    session_id: &str,
    client: &ClientWithMiddleware,
    lock: Arc<Mutex<()>>,
) -> Result<(), anyhow::Error> {
    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    // let mut data: Orders = serde_json::from_str(&contents)?;

    // info!("Data: {contents}");

    /*
    let data = &mut quick_xml::de::Deserializer::from_str(&contents);

    let mut data: Orders = match serde_path_to_error::deserialize(data) {
        Ok(data) => data,
        Err(err) => return Err(anyhow!("Failed to Deserialize Order: {err}")),
    };
    debug!("Data before operations: {data:?}");
    */

    let json_config = Config::new_with_custom_values(true, "", "txt", NullValue::Null);

    let json_struct = match xml_str_to_json(&contents, &json_config) {
        Ok(json) => json,
        Err(err) => {
            error!("Failed to convert XML to JSON: {err}");
            return Err(anyhow!("Failed to convert XML to JSON: {err}"));
        }
    };

    let json = match extract_root_data(json_struct) {
        Ok(json) => json,
        Err(err) => panic!("Failed to Extract Root Data from JSON, {err}"),
    };

    // Some Deserializer.
    let json_str = json.to_string();
    let mut de = serde_json::Deserializer::from_str(&json_str);
    debug!("Converted JSON: {}", json.to_string());

    // Deserialize with serde_path_to_error
    let mut data: Orders = match serde_path_to_error::deserialize(&mut de) {
        Ok(d) => d,
        Err(err) => {
            // err.path() gives the JSON path where the error occurred
            error!("Error JSON body: {json_str}");
            return Err(anyhow!(
                "Failed to deserialize JSON at {}: {}",
                err.path(),
                err
            ));
        }
    };

    let doc_num = data.doc_num.ok_or(anyhow!("No DocNum in file"))?;

    let (doc_entry, expenses, whsecode) =
    get_order_doc_entry(&doc_num, session_id, client, &mut data, Arc::clone(sap_pool)).await?;
    data = update_base_entries(data, doc_entry, &whsecode).await;
    data.document_additional_expenses = Some(expenses);

    let (tracking_data, del_entry) = {
        let _guard = lock.lock().await;

        // Only one task at a time will run this part
        data.document_lines =
            match generate_batchnumbers(&data.document_lines, Arc::clone(sap_pool)).await {
                Ok(document_lines) => document_lines,
                Err(err) => {
                    error!("Error Generating Batch Numbers: {err}");
                    return Err(anyhow!("Error Generating Batch Numbers: {err}"));
                }
            };

        let tracking_data = data.tracking.take().ok_or(anyhow!("No tracking data"))?;
        info!("{}", serde_json::to_string(&data).unwrap());
        let del_entry = post_delivery(&data, session_id, client).await?;

        // Assign to tuple → values can escape after lock is dropped
        (tracking_data, del_entry)
    };

    /*
    data.document_lines =
        match generate_batchnumbers(&data.document_lines, Arc::clone(sap_pool)).await {
            Ok(document_lines) => document_lines,
            Err(err) => {
                error!("Error Generating Batch Numbers: {err}");
                return Err(anyhow!("Error Generating Batch Numbers: {err}"));
            }
        };
    let tracking_data = data.tracking.take().ok_or(anyhow!("No tracking data"))?;
    // if !data.freight_expense.is_none(){
    //        let json_payload = json!({
    //             "DocumentAdditionalExpenses": [
    //         {
    //             "ExpenseCode": expensecode,
    //             "LineTotal": value
    //         }
    //     ]
    // });
    // }
    info!("{}", serde_json::to_string(&data).unwrap());
    let del_entry = post_delivery(data, session_id, client).await?;
    */

    info!("Tracking data before update: {:?}", tracking_data);
    let tracking_data: Vec<Tracking> = if ["PILEU", "PIL"].contains(&whsecode.as_str())  {
        let tracking_data = get_carton_quantity_pil(&data.document_lines, &tracking_data, Arc::clone(sap_pool)).await?;
        info!("PIL Tracking data before update: {:?}", tracking_data);
        update_doc_entries_tracking(tracking_data, del_entry).await
    } else {
        update_doc_entries_tracking(tracking_data, del_entry).await
    };
    info!("Tracking data after update: {:?}", tracking_data);

    if tracking_data.is_empty() {
        error!("Tracking Data Vec is empty, No tracking data to post to SAP");
        return Err(anyhow!("No tracking data in Tracking Data Vec to post to SAP"));
    }

    let query = generate_insert_query(tracking_data, "[@ECSB1_DLN]").await;
    match send_query(sap_pool, &query, &[&""]).await {
        Ok(_) => {}
        Err(err) => {
            error!("Failed to send query, {err}");
            return Err(anyhow!("Failed to send tracking data query, {err}"));
        }
    };
    /*
    for query in &query {
        info!("Insert Tracking Data Query: {query}");
        debug!("Insert Query: {query}");
        match send_query(sap_pool, query, &[&""]).await {
            Ok(_) => {}
            Err(err) => {
                error!("Failed to send query, {err}");
                return Err(anyhow!("Failed to send tracking data query, {err}"));
            }
        };
    }
    */

    let remarks_query = generate_remarks_query(&doc_num).await;
    info!("Remarks Query = {}", &remarks_query);
    let query_result = match send_query(sap_pool, &remarks_query, &[&""]).await {
        Ok(result) => result,
        Err(err) => {
            error!("Failed to send query, {err}");
            return Err(anyhow!("Failed to send query, {err}"));
        }
    };
    info!("Remarks Query Results = {:?}", &query_result);
    match query_result.first() {
        Some(row) => {
            let tracking_info: &str = row.get("Tracking_Info").unwrap_or_default();
            let body = json!({
                "U_TBD_SI_Remarks": tracking_info
            })
            .to_string();
            info!("Remarks body = {}", &body);
            let mut headers = HeaderMap::new();
            headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
            headers.insert(
                COOKIE,
                HeaderValue::from_str(session_id).expect("Failed to create COOKIE header"),
            );
            let delivery_uri = format!("/b1s/v1/DeliveryNotes({del_entry})");
            let delivery_rem = format!("{BASE_URL}{delivery_uri}");
            match send_request(client, Method::PATCH, delivery_rem, body, headers).await {
                Ok(response) => {
                    info!("Delivery Update Response: {:?}", &response);
                    if response.status().is_success() {
                        // let response_data: Value = response.json().await?;
                    } else {
                        // Log and convert error response
                        error!("Delivery PUpdate failed: {}", response.status());

                        // let status = response.status();
                        let body = response.text().await.unwrap_or_default();
                        let minimized_body =
                            jsonxf::minimize(&body).unwrap_or_else(|_| body.clone());

                        let value = serde_json::from_str::<Value>(&minimized_body)
                            .ok()
                            .and_then(|v| {
                                v.pointer("/error/message/value")
                                    .and_then(|v| v.as_str())
                                    .map(str::to_owned)
                            })
                            .unwrap_or(body);

                        

                        return Err(anyhow!(value));

                        /*
                        return Err(anyhow!(
                            "Request failed with status {}: {}",
                            status,
                            minimized_body
                        ));
                        */
                    }
                }
                Err(err) => {
                    error!("Request error: {err}");
                    return Err(anyhow!(err));
                }
            }
        }
        None => {
            error!("No rows returned for query, {query:?}");
            return Err(anyhow!("No rows returned for query, {query:?}"));
        }
    };
    Ok(())
}

async fn get_token(
    company: &str,
    username: &str,
    password: &str,
    client: &ClientWithMiddleware,
) -> Result<Token, Box<dyn std::error::Error>> {
    let uri = "/b1s/v1/Login";
    let url = format!("{BASE_URL}{uri}");

    let login_data = serde_json::json!({
        "CompanyDB": company,
        "Password": password,
        "UserName": username,
    });

    let login_body = serde_json::to_string_pretty(&login_data)
        .unwrap_or_else(|_| "<failed to serialize login data>".to_string());
    info!("Login request body: {}", login_body);

    // Send the POST request with login data
    let response = match client.post(url).json(&login_data).send().await {
        Ok(response) => {
            if response.status().is_success() {
                response
            } else {
                let status = response.status();
                let body_text = match response.text().await {
                    Ok(t) => t,
                    Err(e) => format!("(failed to read body: {})", e),
                };
                error!(
                    "Failed to get login info, Status Code {}: {}",
                    status,
                    body_text
                );
                return Err(format!(
                    "Failed to get login token, status {}: {}",
                    status, body_text
                )
                .into());
            }
        }
        Err(err) => {
            error!("Failed to get token, {err}");
            return Err(format!("Failed to get token {err}").into());
        }
    };

    info!("Token: {:?}", response);

    // Deserialize the response JSON into the Token struct
    let token: Token = match response.json().await {
        Ok(token) => token,
        Err(err) => {
            return Err(format!("Failed to deserialize JSON for token, {err}").into());
        }
    }; // Parse the response body

    Ok(token) // Return the deserialized token
}

/*
async fn save_shipment_json(
    path: &Path,
    order_data: &Value,
    order: &Orders,
    current_time: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // async fn save_warehouse_order_json(path: &Path, order: &Orders) -> Result<(), Box<dyn Error>> {
    let json_location = path.join(format!(
        "{}-order_{}-{}.json",
        order.card_code.replace(" ", "_").replace(",", ""),
        order.doc_num.unwrap(),
        current_time
    ));

    debug!(
        "JSON File Path: {}",
        json_location.as_os_str().to_str().unwrap()
    );
    write(json_location, &order_data.to_string())
        .await
        .expect("Unable to write JSON file");
    Ok(())
}
*/

async fn get_order_doc_entry(
    doc_num: &u64,
    session_id: &str,
    client: &ClientWithMiddleware,
    order: &mut Orders,
    sap_pool: Arc<Pool<bb8_tiberius::ConnectionManager>>,
) -> Result<(u64, Vec<Value>, String), anyhow::Error> {
    let uri = "/b1s/v1/Orders";
    /*
    let query = format!(
        "$select=DocEntry&$filter=DocNum,U_BillingType,DocumentAdditionalExpenses eq {doc_num}"
    );
    */
    let query = format!(
        "$select=DocEntry,TransportationCode,U_BillingType,DocumentLines,DocumentAdditionalExpenses&$filter=DocNum eq {doc_num}"
    );

    let url = format!("{BASE_URL}{uri}?{query}");
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(
        COOKIE,
        HeaderValue::from_str(session_id).expect("Failed to create COOKIE header"),
    );
    info!("Sending Get Order Doc Entry Request {:?}, {url}", headers);
    let mut additional_expense = Vec::new();
    let mut additional_expense_order = Vec::new();
    match send_request(client, Method::GET, url, String::new(), headers).await {
        Ok(response) => {
            info!("Get Order Doc Entry Response: {:?}", response);
            if response.status().is_success() {
                let response_data: Value = response.json().await?;
                info!(
                    "Order Doc Entry Response Body: {}",
                    response_data.to_string()
                );
                // let order_freight = false;
                let doc_entry = match response_data["value"][0]["DocEntry"].as_u64() {
                    Some(doc_entry) => doc_entry,
                    None => {
                        error!("No Doc Entry in response for {doc_num}");
                        return Err(anyhow!("No Doc Entry in response for {doc_num}"));
                    }
                };

                let warehouse_code = match response_data["value"][0]["DocumentLines"][0]["WarehouseCode"].as_str() {
                    Some(warehouse_code) => warehouse_code,
                    None => {
                        error!("No Warehouse Code in response for {doc_num}");
                        return Err(anyhow!("No Warehouse Code in response for {doc_num}"));
                    }
                };
                // TransportationCode
                // let transportation_code = match response_data["value"][0]["TransportationCode"].as_str() {
                //     Some(transportation_code) => transportation_code,
                //     None => {
                //         error!("No Transportation Code in response for {doc_num}");
                //         return Err(anyhow!("No Transportation Code in response for {doc_num}"));
                //     }
                // };
                let transportation_code = match response_data["value"][0]["TransportationCode"].as_i64() {
                    Some(code) => code.to_string(),
                    None => {
                        error!("No Transportation Code in response for {doc_num}");
                        return Err(anyhow!("No Transportation Code in response for {doc_num}"));
                    }
                };

                // let carrier_type = order.document_lines
                //     .as_ref()
                //     .and_then(|t| t.first())
                //     .and_then(|t| t.u_carrier_code.as_deref())
                //     .unwrap_or("");
                // let carrier_type = order.document_lines
                // .as_deref()
                // .and_then(|t| t.first())
                // .and_then(|t| t.u_carrier_code.as_deref())
                // .unwrap_or("");

                let carrier_type = order.document_lines
                .first()
                .and_then(|t| t.u_carrier_code.as_deref())
                .unwrap_or("");

                let carrier_query = format!("SELECT U_SCAC FROM OSHP WHERE TrnspCode = {}", transportation_code);
                let carrier_result: String = match send_query(&sap_pool, &carrier_query, &[]).await {
                    Ok(rows) => rows
                        .first()
                        .and_then(|row| row.get("U_SCAC"))
                        .map(|v: &str| v.to_string())
                        .unwrap_or_default(),  // returns empty String when no record
                    Err(e) => {
                        eprintln!("Failed to fetch carrier SCAC: {}", e);
                        String::new()
                    }
                };

                if carrier_type != carrier_result {
                    error!("Carrier Code Mismatch: order={}, sap={}", carrier_type, carrier_result);
                    return Err(anyhow!("Carrier Code Mismatch: order={}, sap={}", carrier_type, carrier_result));
                }

                if warehouse_code != "PILEU" && warehouse_code != "PIL" {
                    if let Some(billing_type) = response_data["value"][0]["U_BillingType"].as_str() {
                        let billing_type = billing_type.trim();
                        info!("Billing Type: {}", billing_type);
                        
                        // if order.billing_type != billing_type {
                        //     error!("Billing Type Mismatch: {}", billing_type);
                        //     return Err(anyhow!("Billing Type Mismatch: {}", billing_type));
                        // }
                        match &order.billing_type {
                            Some(order_bt)  => {
                            let order_bt = order_bt.trim();
                            info!("Billing Type Match: {}", order_bt);
                                if order_bt == billing_type || order_bt == "PREPAID BILL" || order_bt == "PREPAID" {
                                    info!("Billing Type Match: order={}, sap={}", order_bt, billing_type);
                                }
                            // }
                            // Some(order_bt) => {
                            else {
                                error!(
                                    "Billing Type Mismatch: order={}, sap={}",
                                    order_bt,
                                    billing_type
                                );
                                return Err(anyhow!(
                                    "Billing Type Mismatch: order={}, sap={}",
                                    order_bt,
                                    billing_type
                                ));
                            }
                        }
                            // }
                            None => {
                                // return Err(anyhow!("Missing billing_type in order"));
                            }
                        }
                    } else {
                        return Err(anyhow!("Missing U_BillingType"));
                    }
                }
                // if let Some(expenses) =
                //     response_data["value"][0]["DocumentAdditionalExpenses"].as_array()
                // {
                //     additional_expense = expenses.to_vec();
                //     for expense in expenses {
                //         if let (Some(expense_code), Some(_line_total)) = (
                //             expense["ExpenseCode"].as_i64(),
                //             expense["LineTotal"].as_str(),
                //         ) && expense_code == 1
                //         {
                //             order_freight = true;
                //         }
                //     }
                // }

                // if !order_freight
                //     && let Some(billing_type) = response_data["value"][0]["U_BillingType"].as_str()
                //     && billing_type == "PREPAID BILL"
                // if order.freight_expense.as_deref().map_or(false, |v| !v.trim().is_empty()){
                //     let uri = format!("/b1s/v1/Orders({doc_entry})");
                //     let url = format!("{BASE_URL}{uri}");
                //     let mut headers = HeaderMap::new();
                //     headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                //     headers.insert(
                //         COOKIE,
                //         HeaderValue::from_str(session_id).expect("Failed to create COOKIE header"),
                //     );
                //     match send_request(
                //         client,
                //         Method::PATCH,
                //         url,
                //         json_payload.to_string(),
                //         headers,
                //     )
                //     .await
                //     {
                //     }
                // }

                let shipping_multiplier = format!("
                SELECT TOP 1 U_FreightUpcharge
                FROM [@ECSB1_FRT_UPCHARGE]
                WHERE Code IN ('{code}', 'DEFAULT')
                ORDER BY CASE
                WHEN Code = '{code}' THEN 1
                WHEN Code = 'DEFAULT' THEN 2
                END", code=order.card_code
                );
              
                // let freight_upcharge: f64 = match send_query(&sap_pool, &shipping_multiplier, &[]).await {
                //     Ok(rows) => rows
                //         .first()
                //         .and_then(|row| row.try_get::<f64, _>("U_FreightUpcharge").ok().flatten())  // 👈 flatten
                //         .unwrap_or(1.0),
                //     Err(e) => {
                //         eprintln!("Failed to fetch freight upcharge: {}", e);
                //         1.0
                //     }
                // };
                // let freight_upcharge: f64 = match send_query(&sap_pool, &shipping_multiplier, &[]).await {
                //     Ok(rows) => {
                //         info!("Freight upcharge rows count: {}", rows.len());
                //         let first = rows.first();
                //         info!("First row: {:?}", first);
                        
                //         let result = first
                //             .and_then(|row| {
                //                 let val = row.try_get::<f64, _>("U_FreightUpcharge");
                //                 info!("try_get result: {:?}", val);
                //                 val.ok().flatten()
                //             })
                //             .unwrap_or(1.0);
                        
                //         info!("Final freight_upcharge: {}", result);
                //         result
                //     },
                //     Err(e) => {
                //         eprintln!("Failed to fetch freight upcharge: {}", e);
                //         1.0
                //     }
                // };
                let freight_upcharge: f64 = match send_query(&sap_pool, &shipping_multiplier, &[]).await {
                    Ok(rows) => {
                        info!("Freight upcharge rows count: {}", rows.len());
                        let result = rows
                            .first()
                            .and_then(|row| {
                                let val = row.try_get::<rust_decimal::Decimal, _>("U_FreightUpcharge");
                                info!("try_get result: {:?}", val);
                                val.ok().flatten()
                            })
                            .map(|d| d.to_string().parse::<f64>().unwrap_or(1.0))
                            .unwrap_or(1.0);
                        info!("Final freight_upcharge: {}", result);
                        result
                    },
                    Err(e) => {
                        eprintln!("Failed to fetch freight upcharge: {}", e);
                        1.0
                    }
                };
                
                info!("Freight Upcharge Multiplier: {}", freight_upcharge);
                let mut has_expense_code_1 = false;
                // let mut finalized_freight:String = String::new();
                // if freight_upcharge != 1.0 && freight_upcharge > 0.0 {
                //     let original_expense = order.freight_expense
                //         .as_deref()
                //         .and_then(|v| v.parse::<f64>().ok())
                //         .unwrap_or(0.0);
                //     let upcharged_expense = original_expense * freight_upcharge;
                //     finalized_freight = upcharged_expense.to_string();
                //     info!("Applied freight upcharge: original={}, multiplier={}, upcharged={}", original_expense, freight_upcharge, upcharged_expense);
                // }
                // else{
                //     finalized_freight = order.freight_expense.clone().unwrap_or_default();
                // }
                let mut original_expense : f64 = 0.0;
                let finalized_freight: String = if order.freight_expense.as_deref().is_some_and(|v| !v.trim().is_empty()) {
                    info!("Original Freight Expense: {}", order.freight_expense.as_deref().unwrap_or_default());
                    if freight_upcharge != 1.0 && freight_upcharge > 0.0 {
                        original_expense = order.freight_expense
                            .as_deref()
                            .and_then(|v| v.parse::<f64>().ok())
                            .unwrap_or(0.0);
                        let upcharged_expense = original_expense * freight_upcharge;
                        info!("Applied freight upcharge: original={}, multiplier={}, upcharged={}", 
                            original_expense, freight_upcharge, upcharged_expense);
                        upcharged_expense.to_string()
                    } else {
                        order.freight_expense.clone().unwrap_or_default()
                    }
                } else {
                    String::new()
                };
                // order.freight_expense = if finalized_freight.is_empty() {
                //     None
                // } else {
                //     Some(finalized_freight.clone())
                // };
                if order
                    .freight_expense
                    .as_deref()
                    .is_some_and(|v| !v.trim().is_empty())
                {
                    
                    let json_payload = json!({
                        "U_Freight_Expense": original_expense
                    });
                    let uri = format!("/b1s/v1/Orders({doc_entry})");
                    let url = format!("{BASE_URL}{uri}");
                    let mut headers = HeaderMap::new();
                    info!(
                        "sending PATCH request to Sales Order for the expense update {}",
                        &json_payload
                    );
                    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                    headers.insert(
                        COOKIE,
                        HeaderValue::from_str(session_id).expect("Failed to create COOKIE header"),
                    );
                    match send_request(
                        client,
                        Method::PATCH,
                        url,
                        json_payload.to_string(),
                        headers,
                    )
                    .await
                    {
                        Ok(_) => (),
                        Err(err) => {
                            error!("Failed to patch order for freight code, {err}");
                            return Err(err.into());
                        }
                    }
                }

                if let Some(expenses) =
                    response_data["value"][0]["DocumentAdditionalExpenses"].as_array()
                    && !expenses.is_empty()
                {
                    info!("Additional Expenses addess");
                    additional_expense = expenses.to_vec();
                    additional_expense_order = expenses.to_vec();
                    for expense in additional_expense.iter_mut() {
                        if let Some(obj) = expense.as_object_mut() {
                            obj.insert("BaseDocEntry".to_string(), json!(doc_entry));
                            if let Some(line_num) = obj.get("LineNum") {
                                obj.insert("BaseDocLine".to_string(), line_num.clone());
                            }
                            obj.insert("BaseDocType".to_string(), json!(17));
                        } else {
                            // Log if we expected an object but didn't get one
                            warn!(
                                "Expected object in additional expenses array, got: {:?}",
                                expense
                            );
                        }
                    }
                    //     if let Ok(json_str) = to_string_pretty(&additional_expense) {
                    //         info!("Updated DocumentAdditionalExpenses:\n{}", json_str);
                    //     }
                }

                if let Some(billing_type) = response_data["value"][0]["U_BillingType"].as_str()
                    && billing_type == "PREPAID BILL"
                    && let Some(expenses) =
                        response_data["value"][0]["DocumentAdditionalExpenses"].as_array()
                {
                    let shipping_type = order.tracking
                    .as_ref()
                    .and_then(|t| t.first())
                    .and_then(|t| t.u_pack_level_type.as_deref())
                    .unwrap_or("");
                    
                    if billing_type == "PREPAID BILL" && shipping_type == "U" {
                        let has_freight = order.freight_expense
                            .as_deref()
                            .is_some_and(|v| !v.trim().is_empty());

                        if !has_freight {
                            // order has PREPAID BILL + U shipping but NO freight charge
                            // info!("Order has PREPAID BILL and U shipping but no freight expense");
                             return Err(anyhow!(
                                        "Order has PREPAID BILL and U shipping but no freight expense"
                                    ));
                        } 
                        // else {
                        //     info!("Order has PREPAID BILL and U shipping with freight expense included");
                        // }
                    }

                    if !expenses.is_empty() {
                        additional_expense = expenses.to_vec();
                        for expense in additional_expense.iter_mut() {
                            if let Some(obj) = expense.as_object_mut() {
                                if obj.get("ExpenseCode").and_then(|code| code.as_i64()) == Some(1)
                                {
                                    has_expense_code_1 = true;
                                }
                                obj.insert("BaseDocEntry".to_string(), json!(doc_entry));
                                if let Some(line_num) = obj.get("LineNum") {
                                    obj.insert("BaseDocLine".to_string(), line_num.clone());
                                }
                                obj.insert("BaseDocType".to_string(), json!(17));
                            } else {
                                // Log if we expected an object but didn't get one
                                warn!(
                                    "Expected object in additional expenses array, got: {:?}",
                                    expense
                                );
                            }
                        }
                        if !has_expense_code_1
                            && order
                                .freight_expense
                                .as_ref()
                                .is_some_and(|v| !v.trim().is_empty())
                        {
                            let length = additional_expense.len();
                         
                            additional_expense.push(json!({
                            "ExpenseCode": 1,
                            "LineTotal": &finalized_freight,
                            "BaseDocEntry": doc_entry,
                            "BaseDocLine": &length,
                            "BaseDocType": 17
                            }));
                            additional_expense_order.push(json!({
                                "ExpenseCode": 1,
                                "LineTotal": &finalized_freight,
                                "LineNum": &length

                            }));
                            let addition_exp = json!({
                                "DocumentAdditionalExpenses": &additional_expense_order
                            });
                            let additional_expense_json = match serde_json::to_string(&addition_exp)
                            {
                                Ok(json) => json,
                                Err(err) => {
                                    error!(
                                        "Failed to convert additional_expense {additional_expense:?} to json: {err}"
                                    );
                                    return Err(anyhow!(
                                        "Failed to convert additional_expense {additional_expense:?} to json: {err}"
                                    ));
                                }
                            };
                            info!(
                                "sending PATCH request to Sales Order for the existing expense update {}",
                                &additional_expense_json
                            );
                            let uri = format!("/b1s/v1/Orders({doc_entry})");
                            let url = format!("{BASE_URL}{uri}");
                            let mut headers = HeaderMap::new();
                            headers
                                .insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                            headers.insert(
                                COOKIE,
                                HeaderValue::from_str(session_id)
                                    .expect("Failed to create COOKIE header"),
                            );
                            match send_request(
                                client,
                                Method::PATCH,
                                url,
                                additional_expense_json,
                                headers,
                            )
                            .await
                            {
                                Ok(_) => {
                                    return Ok((doc_entry, additional_expense, String::new()));
                                }
                                Err(err) => {
                                    error!("Failed to patch order, {err}");
                                    return Err(err.into());
                                }
                            }
                        }
                        // if let Ok(json_str) = to_string_pretty(&additional_expense) {
                        //     info!("Updated DocumentAdditionalExpenses:\n{}", json_str);
                        // }
                    } else if order
                        .freight_expense
                        .as_deref()
                        .is_some_and(|v| !v.trim().is_empty())
                    {
                        let json_payload = json!({
                                    "DocumentAdditionalExpenses": [
                                {
                                    "ExpenseCode": 1,
                                    "LineTotal": &finalized_freight,
                                }
                            ]
                        });
                        additional_expense.push(json!({
                            "ExpenseCode": 1,
                            "LineTotal": &finalized_freight,
                            "BaseDocEntry": doc_entry,
                            "BaseDocLine": 0,
                            "BaseDocType": 17
                        }));
                        info!(
                            "sending PATCH request to Sales Order for the expense update {}",
                            &json_payload
                        );
                        let uri = format!("/b1s/v1/Orders({doc_entry})");
                        let url = format!("{BASE_URL}{uri}");
                        let mut headers = HeaderMap::new();
                        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                        headers.insert(
                            COOKIE,
                            HeaderValue::from_str(session_id)
                                .expect("Failed to create COOKIE header"),
                        );
                        match send_request(
                            client,
                            Method::PATCH,
                            url,
                            json_payload.to_string(),
                            headers,
                        )
                        .await
                        {
                            Ok(_) => {
                                return Ok((doc_entry, additional_expense, String::new()));
                            }
                            Err(err) => {
                                error!("Failed to patch order, {err}");
                                return Err(err.into());
                            }
                        }
                    }
                }
                Ok((doc_entry, additional_expense, warehouse_code.to_string()))
            } else {
                error!("Order not found : {}", response.status());
                Err(response.error_for_status().unwrap_err().into())
            }
        }
        Err(err) => {
            error!("Failed to get doc order entry, {err}");
            Err(err.into())
        }
    }
}

async fn update_base_entries(mut orders: Orders, base_entry: u64, warehouse: &str) -> Orders {
    for document_line in &mut orders.document_lines {
        document_line.base_entry = base_entry;
        if document_line.warehouse_code == Some("PIL UK".to_string()) {
            document_line.warehouse_code = Some(warehouse.to_string());
        }
    }
    orders.doc_num = None;
    orders
}

pub async fn update_doc_entries_tracking(
    mut tracking_list: Vec<Tracking>,
    new_doc_entry: u64,
) -> Vec<Tracking> {
    for tracking in tracking_list.iter_mut() {
        tracking.u_doc_entry = Some(new_doc_entry);
    }
    tracking_list
}

async fn post_delivery(
    order: &Orders,
    session_id: &str,
    client: &ClientWithMiddleware,
) -> Result<u64, anyhow::Error> {
    let uri = "/b1s/v1/DeliveryNotes";
    // let query = format!("$select=DocEntry&$filter=DocNum eq '{}'", doc_num);
    let url = format!("{BASE_URL}{uri}");
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(
        COOKIE,
        HeaderValue::from_str(session_id).expect("Failed to create COOKIE header"),
    );

    let order_string = match serde_json::to_string(&order) {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to serialize data: {}", e);
            return Err(anyhow!("Failed to serialize delivery data"));
        }
    };

    info!("Sending Post Delivery Request {:?}, {url}", headers);
    info!("Post Delivery Body: {order_string}");
    // let response = match client.get(url).headers(headers).send().await {
    match send_request(client, Method::POST, url, order_string, headers).await {
        Ok(response) => {
            info!("Post Delivery Response: {:?}", response);
            if response.status().is_success() {
                let response_data: Value = response.json().await?;
                response_data["DocEntry"]
                    .as_u64()
                    .ok_or_else(|| anyhow!("DocEntry not found or invalid in response"))
            } else {
                // Log and convert error response
                error!("Delivery Post failed: {}", response.status());

                // let status = response.status();
                let body = response.text().await.unwrap_or_default();
                let minimized_body = jsonxf::minimize(&body).unwrap_or_else(|_| body.clone());

                let value = serde_json::from_str::<Value>(&minimized_body)
                    .ok()
                    .and_then(|v| {
                        v.pointer("/error/message/value")
                            .and_then(|v| v.as_str())
                            .map(str::to_owned)
                    })
                    .unwrap_or(minimized_body);

                

                Err(anyhow!(value))
                /*
                Err(anyhow!(
                    "Request failed with status {}: {}",
                    status,
                    value
                ))
                */
            }
        }
        Err(err) => {
            error!("Request error: {err}");
            Err(anyhow!(err))
        }
    }
}

async fn send_request(
    client: &ClientWithMiddleware,
    method: reqwest_middleware::reqwest::Method,
    url: String,
    body: String,
    headers: HeaderMap,
) -> Result<Response, reqwest_middleware::Error> {
    match method {
        Method::GET => {
            let mut request = client.get(&url);
            if !body.is_empty() {
                request = request.body(body);
            }
            if !headers.is_empty() {
                request = request.headers(headers);
            }
            request.send().await
        }
        Method::POST => {
            let mut request = client.post(&url).body(body);
            if !headers.is_empty() {
                request = request.headers(headers);
            }
            request.send().await
        }
        Method::PATCH => {
            let mut request = client.patch(&url).body(body);
            if !headers.is_empty() {
                request = request.headers(headers);
            }
            request.send().await
        }
        Method::PUT => {
            let mut request = client.put(&url);
            if !body.is_empty() {
                request = request.body(body);
            }
            if !headers.is_empty() {
                request = request.headers(headers);
            }
            request.send().await
        }
        Method::HEAD => {
            let mut request = client.head(&url);
            if !body.is_empty() {
                request = request.body(body);
            }
            if !headers.is_empty() {
                request = request.headers(headers);
            }
            request.send().await
        }
        _ => {
            info!("No Method Provided, Defaulting to GET Method for {url}");
            let mut request = client.get(&url);
            if !body.is_empty() {
                request = request.body(body);
            }
            if !headers.is_empty() {
                request = request.headers(headers);
            }
            request.send().await
        }
    }
}

async fn generate_remarks_query(doc_num: &u64) -> String {
    let query = format!(
        "
        DECLARE @SO_Num INT = {doc_num};

        SELECT TOP 1 CASE
                -- Pallet level
                WHEN dln3.u_packleveltype = 'P' THEN
                'BOL Num: ' + dln.u_bol_nbr + ' CartonCnt: '
                + Cast(dln3.u_cartonquantity AS NVARCHAR)
                -- Carton level - BOL only
                WHEN dln3.u_packleveltype = 'C'
                        AND dln.u_bol_nbr IS NOT NULL
                        AND dln.u_pro_nbr IS NULL THEN
                'BOL Num: ' + dln.u_bol_nbr + ' CartonCnt: '
                + Cast( (SELECT Sum(dln3_inner.u_cartonquantity) FROM
                [@ecsb1_dln] dln3_inner
                INNER JOIN dln1 dln_inner ON
                dln_inner.docentry = dln3_inner.u_docentry AND dln_inner.linenum
                =
                dln3_inner.u_linenum INNER JOIN rdr1 so_line_inner ON
                dln_inner.baseentry =
                so_line_inner.docentry AND dln_inner.baseline =
                so_line_inner.linenum INNER
                JOIN ordr
                so_inner ON so_inner.docentry = so_line_inner.docentry WHERE
                so_inner.docnum =
                @SO_Num AND dln3_inner.u_packleveltype = 'C' ) AS NVARCHAR)
                -- Carton level - PRO only
                WHEN dln3.u_packleveltype = 'C'
                        AND dln.u_bol_nbr IS NULL
                        AND dln.u_pro_nbr IS NOT NULL THEN
                'U_PRO_NBR: ' + dln.u_pro_nbr + ' CartonCnt: '
                + Cast( (SELECT Sum(dln3_inner.u_cartonquantity) FROM
                [@ecsb1_dln] dln3_inner
                INNER JOIN dln1 dln_inner ON
                dln_inner.docentry = dln3_inner.u_docentry AND dln_inner.linenum
                =
                dln3_inner.u_linenum INNER JOIN rdr1 so_line_inner ON
                dln_inner.baseentry =
                so_line_inner.docentry AND dln_inner.baseline =
                so_line_inner.linenum INNER
                JOIN ordr
                so_inner ON so_inner.docentry = so_line_inner.docentry WHERE
                so_inner.docnum =
                @SO_Num AND dln3_inner.u_packleveltype = 'C' ) AS NVARCHAR)
                -- Carton level - BOL and PRO both present
                WHEN dln3.u_packleveltype = 'C'
                        AND dln.u_bol_nbr IS NOT NULL
                        AND dln.u_pro_nbr IS NOT NULL THEN
                'BOL Num: ' + dln.u_bol_nbr + ' PRO_NBR: '
                + dln.u_pro_nbr + ' CartonCnt: '
                + Cast( (SELECT Sum(dln3_inner.u_cartonquantity) FROM
                [@ecsb1_dln] dln3_inner
                INNER JOIN dln1 dln_inner ON
                dln_inner.docentry = dln3_inner.u_docentry AND dln_inner.linenum
                =
                dln3_inner.u_linenum INNER JOIN rdr1 so_line_inner ON
                dln_inner.baseentry =
                so_line_inner.docentry AND dln_inner.baseline =
                so_line_inner.linenum INNER
                JOIN ordr
                so_inner ON so_inner.docentry = so_line_inner.docentry WHERE
                so_inner.docnum =
                @SO_Num AND dln3_inner.u_packleveltype = 'C' ) AS NVARCHAR)
                -- Small Parcel - Tracking only
                WHEN dln3.u_packleveltype = 'U' THEN
                'TRACKING: ' + dln3.u_trackingid
                + ' CartonCnt: '
                + Cast(dln3.u_cartonquantity AS NVARCHAR)
                END AS Tracking_Info
    FROM   ordr so
        INNER JOIN rdr1 so_line
                ON so.docentry = so_line.docentry
        LEFT JOIN dln1 dln
                ON dln.baseentry = so_line.docentry
                    AND dln.baseline = so_line.linenum
                    AND dln.basetype = 17
        LEFT JOIN odln dn
                ON dn.docentry = dln.docentry
        LEFT JOIN [@ecsb1_dln] dln3
                ON dln.docentry = dln3.u_docentry
                    AND dln.linenum = dln3.u_linenum
    WHERE  so.docnum = @SO_Num 
"
    );
    // let query = format!(
    //     "DECLARE @SO_Num INT = {doc_num};
    //     SELECT top 1 
    //     CASE	
    //         --
    //         WHEN dln3.U_PackLevelType = 'P'		THEN 'BOL Num: ' + dln.U_BOL_NBR + ' CartonCnt: ' + CAST(dln3.U_CartonQuantity AS nvarchar)
    //         --The BOL is not NULL 
    //         WHEN dln3.U_PackLevelType = 'C'		
    //             AND dln.U_BOL_NBR IS NOT NULL
    //             AND dln.U_PRO_NBR IS NULL		THEN 'BOL Num: ' + dln.U_BOL_NBR + ' CartonCnt: ' + CAST(dln3.U_CartonQuantity AS nvarchar)
    //         --When BOL is NULL
    //         WHEN dln3.U_PackLevelType = 'C'		
    //             AND dln.U_BOL_NBR IS NULL		
    //             AND dln.U_PRO_NBR IS NOT NULL	THEN 'U_PRO_NBR: ' + dln.U_PRO_NBR + ' CartonCnt: ' + CAST(dln3.U_CartonQuantity AS nvarchar)
    //         --BOL and PRO present
    //         WHEN dln3.U_PackLevelType = 'C'		
    //             AND dln.U_BOL_NBR IS NOT NULL
    //             AND dln.U_PRO_NBR IS NOT NULL	THEN 'BOL Num: ' + dln.U_BOL_NBR + ' PRO_NBR: ' + dln.U_PRO_NBR + ' CartonCnt: ' + CAST(dln3.U_CartonQuantity AS nvarchar)
    //         --Small Parcel Tracking only
    //         WHEN dln3.U_PackLevelType = 'U'		THEN 'TRACKING: ' + dln3.U_TrackingID + ' CartonCnt: ' + CAST(dln3.U_CartonQuantity AS nvarchar)
    //     END AS Tracking_Info
    //     FROM ORDR so
    //     INNER JOIN RDR1 so_line
    //         ON so.DocEntry = so_line.DocEntry
    //     LEFT JOIN DLN1 dln
    //         ON dln.BaseEntry = so_line.DocEntry
    //     AND dln.BaseLine = so_line.LineNum
    //     AND dln.BaseType = 17       -- 17 = Sales Order
    //     LEFT JOIN ODLN dn
    //         ON dn.DocEntry = dln.DocEntry
    //     LEFT JOIN [@ECSB1_DLN] dln3
    //         ON dln.DocEntry = dln3.U_DocEntry AND dln.LineNum = dln3.U_LineNum
    //     WHERE so.DocNum =  @SO_Num"
    // );
    query
}

async fn generate_insert_query(tracking_list: Vec<Tracking>, table_name: &str) -> String {
    let mut query = format!(
        "INSERT INTO {table_name} (U_LineNum, U_ItemCode, U_Quantity, U_CartonID, U_ItemsPerCarton, U_CartonQuantity, U_TrackingID, U_UCC128, U_DocEntry, U_PackLevelType, U_UPalletSSCC, U_ShippingType, U_EstDeliveryDate, U_PalletCount)
        SELECT * FROM (VALUES"
    );

    let mut values: Vec<String> = Vec::new();

    for t in tracking_list {
        values.push(format!(
            "({}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {})",
            t.u_line_num.map_or("NULL".to_string(), |v| v.to_string()),
            t.u_item_code
                .as_ref()
                .map_or("NULL".to_string(), |v| ["'", v, "'"].concat()),
            t.u_quantity.map_or("NULL".to_string(), |v| v.to_string()),
            t.u_carton_id.map_or("NULL".to_string(), |v| v.to_string()),
            t.u_items_per_carton
                .map_or("NULL".to_string(), |v| v.to_string()),
            t.u_carton_quantity
                .map_or("NULL".to_string(), |v| v.to_string()),
            t.u_tracking_id
                .map_or("NULL".to_string(), |v| ["'", &v, "'"].concat()),
            t.u_ucc128
                .map_or("NULL".to_string(), |v| ["'", &v, "'"].concat()),
            t.u_doc_entry.map_or("NULL".to_string(), |v| v.to_string()),
            t.u_pack_level_type
                .as_ref()
                .map_or("NULL".to_string(), |v| ["'", v, "'"].concat()),
            t.u_upallet_sscc
                .map_or("NULL".to_string(), |v| ["'", &v, "'"].concat()),
            t.u_shipping_type
                .map_or("NULL".to_string(), |v| ["'", &v, "'"].concat()),
            t.u_est_del
                .map_or("NULL".to_string(), |v| ["'", &v, "'"].concat()),
            t.u_pallet_count
                .map_or("NULL".to_string(), |v| ["'", &v.to_string(), "'"].concat()),
            // .map_or("NULL".to_string(), |v| format!("'{v}'")),
        ));
    }
    query.push_str(&values.join(",\n"));
    query.push_str(") AS t (U_LineNum, U_ItemCode, U_Quantity, U_CartonID, U_ItemsPerCarton, U_CartonQuantity, U_TrackingID, U_UCC128, U_DocEntry, U_PackLevelType, U_UPalletSSCC, U_ShippingType, U_EstDeliveryDate, U_PalletCount);");

    query
}

async fn get_carton_quantity_pil(
    document_lines: &[DocumentLine],
    tracking_lines: &[Tracking],
    sap_pool: Arc<Pool<bb8_tiberius::ConnectionManager>>,
) -> Result<Vec<Tracking>, anyhow::Error> {
    let item_codes: Vec<String> = document_lines
        .iter()
        .map(|line| format!("'{}'", line.item_code)) // add single quotes for SQL
        .collect();
    let item_code_list = format!("({})", item_codes.join(", "));
    let query = format!(
        "SELECT
        ItemCode,
        PurPackUn
    FROM
        OITM
    WHERE
        ItemCode IN {item_code_list}"
    );
    info!("PIL Cartoon Quantity Query {}", query);
    let query_result = match send_query(&sap_pool, &query, &[&""]).await {
        Ok(rows) => rows,
        Err(err) => {
            error!("Failed to send query for batch numbers, {err}");
            return Err(anyhow!("Failed to send query for batch numbers, {err}"));
        }
    };

    info!("PIL Cartoon Quantity Query Result {:?}", query_result);
    let doc_line_map: HashMap<&str, &DocumentLine> = document_lines
        .iter()
        .map(|line| (line.item_code.as_str(), line))
        .collect();

    let mut updated_tracking = tracking_lines.to_vec();
    for row in query_result {
        let item_code: &str = row.get("ItemCode").unwrap_or_default();
        let case_pack_qty: f64 = row.get::<Decimal, _>("PurPackUn").unwrap_or_default().to_string()
        .parse::<f64>()
        .unwrap_or(1.0);
        if let Some(doc_line) = doc_line_map.get(item_code) {
            let quantity = doc_line.quantity;
            info!(
                "Item: {}, CasePack: {}, Quantity: {}",
                item_code, case_pack_qty, quantity
            );

            for tracking in updated_tracking.iter_mut() {
                if tracking
                    .u_item_code
                    .as_deref()
                    .map(|s| s.trim().to_lowercase())
                    == Some(item_code.trim().to_lowercase())
                {
                    tracking.u_carton_quantity =
                        Some((quantity as f64 / case_pack_qty).ceil() as i64);
                    tracking.u_items_per_carton =
                        Some(case_pack_qty.ceil() as i64);
                }
            }

            /*
            for tracking in updated_tracking.iter_mut() {
                if tracking.u_item_code == Some(item_code.to_string()) {
                    // tracking.u_carton_quantity = Some((quantity as f64 / case_pack_qty.parse::<f64>().unwrap()).ceil() as i64);
                    tracking.u_carton_quantity = Some((quantity as f64 / case_pack_qty).ceil() as i64);
                    tracking.u_items_per_carton = Some(case_pack_qty.ceil() as i64);

                }
            }
            */
        }

    }
    Ok(updated_tracking)
}

async fn generate_batchnumbers(
    document_lines: &[DocumentLine],
    sap_pool: Arc<Pool<bb8_tiberius::ConnectionManager>>,
) -> Result<Vec<DocumentLine>, anyhow::Error> {
    let item_codes: Vec<String> = document_lines
        .iter()
        .map(|line| format!("'{}'", line.item_code)) // add single quotes for SQL
        .collect();
    let item_code_list = format!("({})", item_codes.join(", "));

    let warehouse_code = match &document_lines[0].warehouse_code {
        Some(warehouse_code) => warehouse_code,
        None => {
            // error!("No warehouse code found for {:?}", document_lines[0]);
            return Err(anyhow!(
                "No warehouse code found for {:?}",
                document_lines[0]
            ));
        }
    };

    // let warehouse_code = &document_lines[0].warehouse_code.clone().unwrap();

    let query = format!(
        "SELECT
        B.ItemCode,
        B.SysNumber,
        B.DistNumber,
        Q.Quantity,
        Q.WhsCode,
        Q.AbsEntry,
        Q.MdAbsEntry
    FROM
        OBTN B
    JOIN
        OBTQ Q
        ON B.ItemCode = Q.ItemCode AND B.SysNumber = Q.SysNumber
    WHERE
        Q.Quantity > 0
        AND B.ItemCode IN {item_code_list}
        AND Q.WhsCode = '{warehouse_code}'"
    );
    info!("Batch Query {}", query);
    let query_result = match send_query(&sap_pool, &query, &[&""]).await {
        Ok(rows) => rows,
        Err(err) => {
            error!("Failed to send query for batch numbers, {err}");
            return Err(anyhow!("Failed to send query for batch numbers, {err}"));
        }
    };

    info!("Batch Query Result {:?}", query_result);

    // Group results by item_code
    let mut batches_by_item: HashMap<String, Vec<BatchNumbers>> = HashMap::new();

    for row in query_result {
        let item_code: &str = row.get("ItemCode").unwrap_or_default();
        let dist_number: &str = row.get("DistNumber").unwrap_or_default();

        let quantity: i64 = match row.try_get::<Numeric, _>("Quantity") {
            Ok(Some(n)) => {
                let s = n.to_string(); // Convert Numeric to string like "1512.000000"
                match s.parse::<f64>() {
                    Ok(f) => f.trunc() as i64, // Truncate to discard decimal part
                    Err(e) => {
                        eprintln!("Failed to parse Numeric string '{s}': {e}");
                        0
                    }
                }
            }
            Ok(None) => 0, // NULL from SQL
            Err(e) => {
                eprintln!("Failed to get Quantity: {e}");
                0
            }
        };
        // let quantity: i64 = row.get("Quantity").unwrap_or(0);

        let sys_number = row
            .try_get::<i32, _>("SysNumber")
            .map(|v| v.unwrap_or(0) as i64)
            .unwrap_or(0);
        // let sys_number: i64 = row.get("SysNumber").unwrap_or(0);

        batches_by_item
            .entry(item_code.to_owned())
            .or_default()
            .push(BatchNumbers {
                batch_number: Some(dist_number.to_owned()),
                quantity: Some(quantity),
                base_line_number: None, // Will assign later
                u_item_code: Some(item_code.to_owned()),
                system_serial_number: Some(sys_number),
            });
    }

    for batches in batches_by_item.values_mut() {
        batches.sort_by_key(|b| b.system_serial_number);
    }
    let mut updated_lines = Vec::new();

    for line in document_lines.iter() {
        let mut quantity_remaining = line.quantity;
        let mut batch_numbers = Vec::new();
        // info!("Document Line: {:?}", line);

        if let Some(item_batches) = batches_by_item.get(&line.item_code) {
            for batch in item_batches {
                if quantity_remaining <= 0 {
                    break;
                }

                let available_qty = batch.quantity.unwrap_or(0);
                if available_qty <= 0 {
                    continue;
                }

                let used_qty = std::cmp::min(available_qty, quantity_remaining);
                quantity_remaining -= used_qty;

                /*
                info!(
                    "batch_number: {:?},
                    quantity: {},
                    base_line_number: {},
                    u_item_code: {:?},
                    system_serial_number: {:?}",
                    batch.batch_number.clone(),
                    used_qty,
                    line.line_num,
                    batch.u_item_code,
                    batch.system_serial_number
                );
                */

                batch_numbers.push(BatchNumbers {
                    batch_number: batch.batch_number.clone(),
                    quantity: Some(used_qty),
                    base_line_number: Some(line.line_num),
                    u_item_code: batch.u_item_code.clone(),
                    system_serial_number: batch.system_serial_number,
                });
            }
        }

        let mut updated_line = line.clone();
        info!("Batch Numbers: {:?}", batch_numbers);
        updated_line.batch_numbers = Some(batch_numbers);
        updated_lines.push(updated_line);
    }
    Ok(updated_lines)
}

async fn send_query<'b>(
    pool: &Arc<Pool<bb8_tiberius::ConnectionManager>>,
    // conn: &'a mut PooledConnection<'a, ConnectionManager>,
    // pool: &'a Pool<bb8_tiberius::ConnectionManager>,
    query_str: &str,
    // parameters: Vec<u8>,
    // parameters: Vec<SqlType>,
    parameters: &'b [&'b dyn ToSql],
    // ) -> Result<QueryStream, Box<dyn std::error::Error>> {
) -> Result<Vec<Row>, Box<dyn std::error::Error>> {
    debug!("Reaching send_query function with query {query_str}");

    let mut conn = pool
        .get()
        .await
        .expect("Failed to get Pool Connection in send_query function");

    // let stream = match query.query(&mut conn, parameters).await {
    let stream = match conn.query(query_str, parameters).await {
        Ok(stream) => stream,
        Err(err) => {
            error!("SQL Error Response for Query: {query_str}, {err}");
            return Err(Box::new(err));
        }
    };

    return match stream.into_first_result().await {
        Ok(rows) => {
            debug!("Query {query_str} returning {:?}", rows);
            Ok(rows)
        }
        Err(err) => {
            error!("Failed to fetch result for query {query_str}: {err}");
            return Err(Box::new(err));
        }
    };
}

fn extract_root_data(json: serde_json::value::Value) -> Result<Value, Box<dyn std::error::Error>> {
    // Check if the root is an object
    if let Value::Object(root) = json {
        // Take the first key-value pair (if any)
        if let Some((_key, inner_content)) = root.into_iter().next() {
            // Serialize the inner content back to a JSON string
            Ok(inner_content)
        } else {
            Err("Root object is empty".into())
        }
    } else {
        Err("Root is not a JSON object".into())
    }
}
