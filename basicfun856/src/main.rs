mod order_structure;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use quick_xml::se::to_string_with_root;
use anyhow::{anyhow, Result};
use tracing::{info, error, debug, warn};
use tracing_subscriber::EnvFilter;
use clap::Parser;
use chrono::{Local, NaiveDateTime};
use dotenv::dotenv;
use bytes::Bytes;
use std::env;
use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;
use bb8::Pool;
use reqwest_middleware::reqwest::{Client, Method, StatusCode};
// use reqwest_middleware::reqwest::Client;
use reqwest_middleware::reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE, COOKIE};
use tiberius::numeric::Numeric;
use tiberius::{Row, ToSql};
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::{RetryTransientMiddleware, Retryable, policies::ExponentialBackoff};
// use reqwest_middleware::reqwest::StatusCode;
use order_structure::{Orders, Root, Tracking};

const THREADS: usize = 8;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    dropping_path: PathBuf,

    #[arg(short, long)]
    archive_path: PathBuf,

    #[arg(short, long)]
    process_id: String,

    #[arg(short, long)]
    error_process_data_path: PathBuf,
}


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


#[derive(Deserialize, Debug)]
struct Token {
    #[serde(rename = "SessionId")]
    pub session_id: String,
}

async fn get_token(
    base_url: &str,
    company: &str,
    username: &str,
    password: &str,
    client: &ClientWithMiddleware,
) -> Result<Token> {
    let login_data = json!({
        "CompanyDB": company,
        "Password": password,
        "UserName": username,
    });

    let url = format!("{}/Login", base_url);

    info!("--- Login Attempt ---");

    let response = client
        .post(&url)
        .json(&login_data)
        .send()
        .await
        .map_err(|e| anyhow!("Failed to request login: {e}"))?;

    if !response.status().is_success() {
        error!("Failed to get login info, Status Code {}", response.status());
        return Err(anyhow!("Failed to get login token"));
    }

    let token: Token = response
        .json()
        .await
        .map_err(|e| anyhow!("Failed to parse login response: {e}"))?;

    Ok(token)
}


async fn setup_db_pool(env_var: &str) -> Result<Pool<bb8_tiberius::ConnectionManager>> {
    let conn_str = env::var(env_var).expect("SAP_DB_CONN must be set");
    let mgr = bb8_tiberius::ConnectionManager::build(conn_str.as_str())?;
    let pool = bb8::Pool::builder()
        .max_size(THREADS.try_into().unwrap())
        .build(mgr)
        .await?;
    Ok(pool)
}

async fn send_query<'b>(
    pool: &Arc<Pool<bb8_tiberius::ConnectionManager>>,
    query_str: &str,
    parameters: &'b [&'b dyn ToSql],
) -> Result<Vec<Row>, anyhow::Error> {
    debug!("Reaching send_query function with query {query_str}");
    info!("Sending SQL query:\n{}", query_str);

    let mut conn = pool
        .get()
        .await
        .map_err(|err| anyhow!("Failed to get Pool Connection in send_query function: {err}"))?;

    let stream = conn.query(query_str, parameters).await.map_err(|err| {
        error!("SQL Error Response for Query: {query_str}, {err}");
        anyhow!("SQL Error Response for Query: {query_str}, {err}")
    })?;

    let rows = stream.into_first_result().await.map_err(|err| {
        error!("Failed to fetch result for query {query_str}: {err}");
        anyhow!("Failed to fetch result for query {query_str}: {err}")
    })?;

    debug!("Query {query_str} returning rows count {}", rows.len());
    info!("Query response:\n{:#?}", rows);
    Ok(rows)
}

fn row_to_tracking(row: &Row) -> Tracking {
    Tracking {
        u_line_num: row.get::<i32, _>("U_LineNum").map(|v| v as i64),
        u_item_code: row.get::<&str, _>("U_ItemCode").map(str::to_string),
        u_quantity: row.get::<i32, _>("U_Quantity").map(|v| v.to_string()),
        u_carton_id: row.get::<&str, _>("U_CartonID").map(str::to_string),
        // U_ItemsPerCarton comes back as Numericn; read as Numeric and stringify it safely
        u_items_per_carton: row
            .try_get::<Numeric, _>("U_ItemsPerCarton")
            .ok()
            .flatten()
            .map(|v| v.to_string()),
        u_carton_quantity: row.get::<i32, _>("U_CartonQuantity").map(|v| v.to_string()),
        u_tracking_id: row.get::<&str, _>("U_TrackingID").map(str::to_string),
        u_ucc128: row.get::<&str, _>("U_UCC128").map(str::to_string),
        u_doc_entry: row.get::<i32, _>("U_DocEntry").map(|v| v.to_string()),
        u_pack_level_type: row.get::<&str, _>("U_PackLevelType").map(str::to_string),
        u_upallet_sscc: row.get::<&str, _>("U_UPalletSSCC").map(str::to_string),
        u_shipping_type: row.get::<&str, _>("U_ShippingType").map(str::to_string),
        u_lot_number: row.get::<&str, _>("U_LotNumber").map(str::to_string),
        u_pallet_count: row.get::<i32, _>("U_PalletCount").map(|v| v.to_string()),
        // U_EstDeliveryDate is Datetimen; try to read as NaiveDateTime and stringify
        u_est_del: row
            .get::<NaiveDateTime, _>("U_EstDeliveryDate")
            .map(|dt| dt.format("%Y-%m-%dT%H:%M:%S").to_string()),
    }
}

async fn get_tracking_by_doc_entry(
    pool: &Arc<Pool<bb8_tiberius::ConnectionManager>>,
    doc_entry: u64,
) -> Result<Vec<Tracking>, anyhow::Error> {
    let query = format!(
        "SELECT
            U_LineNum,
            U_ItemCode,
            U_Quantity,
            U_CartonID,
            U_ItemsPerCarton,
            U_CartonQuantity,
            U_TrackingID,
            U_UCC128,
            U_DocEntry,
            U_PackLevelType,
            U_UPalletSSCC,
            U_ShippingType,
            U_LotNumber,
            U_PalletCount,
            U_EstDeliveryDate
        FROM [@ECSB1_DLN]
        WHERE U_DocEntry = {}",
        doc_entry
    );

    let rows = send_query(pool, &query, &[&(doc_entry as i64)]).await?;
    Ok(rows.into_iter().map(|row| row_to_tracking(&row)).collect())
}

async fn get_delivery_notes(
    base_url: &str,
    session_id: &str,
    client: &ClientWithMiddleware,
    output_dir: &std::path::Path,
    sap_pool: &Arc<Pool<bb8_tiberius::ConnectionManager>>,
) -> Result<Value> {
    let uri = "/DeliveryNotes";
    let query = "$filter=U_945_Advice eq 'P' AND DocumentStatus eq 'bost_Open'";
    let url = format!("{base_url}{uri}?{query}");

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(
        COOKIE,
        HeaderValue::from_str(session_id).expect("Failed to create COOKIE header"),
    );

    let response = match send_request(client, Method::GET, url.clone(), Bytes::new(), headers.clone()).await {
        Ok(response) => {
            if response.status().is_success() {
                match response.json::<Root>().await {
                    Ok(data) => {
                        debug!("Successfully parsed delivery notes response JSON: {:#?}", data);
                        data.orders
                    },
                    Err(err) => {
                        error!("Failed to parse delivery notes response JSON: {err}");
                        return Err(anyhow!("Failed to parse delivery notes response JSON: {err}"));
                    }
                }
             } else {
                error!("Failed to get delivery notes, status {}", response.status());
                return Err(anyhow!("DeliveryNotes request failed with status {}", response.status()));
            }
        },
        Err(err) => {
            error!("Failed to send request for delivery notes: {err}");
            return Err(anyhow!("Failed to send request for delivery notes: {err}"));
        }
    };

    let order_vec: Vec<serde_json::Value> = response
            .as_array()
            .cloned()
            .expect("Failed to convert order result to array");
    if order_vec.is_empty() {
            // if response.orders.is_empty() {
            warn!("No orders found.");
    } else {
        for mut order in order_vec.into_iter() {
            let now = Local::now();
            let doc_num = order
                .get("DocNum")
                .and_then(|value| value.as_i64())
                .unwrap_or(0);
            let filename = format!("delivery_notes_{}_{}.xml", now.format("%Y%m%d%H%M%S"), doc_num);
            let path = output_dir.join(filename);
                    if let Some(doc_entry) = order.get("DocEntry").and_then(|value| value.as_u64()) {
                match get_tracking_by_doc_entry(sap_pool, doc_entry).await {
                    Ok(tracking_items) if !tracking_items.is_empty() => {
                        if let Some(obj) = order.as_object_mut() {
                            let tracking_value = serde_json::to_value(&tracking_items)
                                .map_err(|e| anyhow!("Failed to serialize tracking data: {e}"))?;
                            obj.insert("Tracking".to_string(), tracking_value);
                        }
                    }
                    Ok(_) => {
                        debug!("No tracking rows found for DocEntry {}", doc_entry);
                    }
                    Err(err) => {
                        warn!("Failed to load tracking for DocEntry {}: {}", doc_entry, err);
                    }
                }
            }

            let mut order_xml = to_string_with_root("root", &order).expect("Failed to serialize to XML");
            order_xml = order_xml
                .replace("\r\n ", " ")
                .replace("\r\n", " ")
                .replace("\r ", " ")
                .replace("\n ", " ")
                .replace(['\r', '\n'], " ");

            tokio::fs::write(&path, order_xml).await.map_err(|e| anyhow!("Unable to write XML file: {e}"))?;
            info!("Saved delivery notes XML to {}", path.display());
        }
    }

    // info!("Saved delivery notes XML to {}", xml_path.display());
    Ok(json!({"status": "success"}))
}

async fn send_request(
    client: &ClientWithMiddleware,
    method: reqwest_middleware::reqwest::Method,
    url: String,
    body: Bytes,
    headers: HeaderMap,
) -> Result<reqwest_middleware::reqwest::Response, reqwest_middleware::Error> {
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


#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // configuring logs
    tracing_subscriber::fmt()
        .with_ansi(false)
        .with_env_filter(EnvFilter::new("info,tiberius=warn"))
        .init();
    dotenv().ok();

    // Clap Args
    let mut args = Args::parse();
    let dropping_path = Arc::new(args.dropping_path);
    let now = Local::now();
    args.archive_path.push(now.format("%Y/%m/%d").to_string());
    let pre_archive_path = Arc::new(args.archive_path);
    let process_id = Arc::new(args.process_id);
    let error_process_data_path = args.error_process_data_path;

    let start = Instant::now();

    let sap_conn_str = std::env::var("SAP_DB_CONN").expect("DB_CONN not found in env");
    let sap_mgr = bb8_tiberius::ConnectionManager::build(sap_conn_str.as_str())?;
    let sap_pool = bb8::Pool::builder()
        .max_size(
            THREADS
                // (THREADS / 2)
                .try_into()
                .expect("Failed to convert THREADS to u32"),
        )
        .build(sap_mgr)
        .await?;
    let sap_pool = Arc::new(sap_pool);

    let base_client = match Client::builder()
        .danger_accept_invalid_certs(true)
        .timeout(Duration::from_secs(60))
        .build()
    {
        Ok(client) => client,
        Err(err) => {
            error!("Failed to build reqwest client: {err}");
            return Err(anyhow!("Failed to build reqwest HTTP client"));
        }
    };

    // Configure retry policy
    let retry_policy = ExponentialBackoff::builder()
        .retry_bounds(Duration::from_millis(100), Duration::from_secs(30))
        .build_with_total_retry_duration(Duration::from_secs(2 * 60));

    let ret_s =
        RetryTransientMiddleware::new_with_policy_and_strategy(retry_policy, FullRetryableStrategy);

    // Wrap with retry middleware
    let client = ClientBuilder::new(base_client).with(ret_s).build();
    let client = Arc::new(client);

    let base_url = env::var("BASE_URL").expect("BASE_URL must be set");
    let company = env::var("Company_DB").expect("Company_DB must be set");
    let username = env::var("User_Name").expect("User_Name must be set");
    let password = env::var("Password").expect("Password must be set");
    info!("env cred: {:?}, {:?}, {:?}, {:?}", base_url, company, username, password);

    let token = get_token(&base_url, &company, &username, &password, &*client).await?;

    let session_id = Arc::new(["B1SESSION=", token.session_id.as_str()].concat());

    // Process delivery notes using DB query
    let dn_resp = get_delivery_notes(&base_url, &session_id, &*client, &*dropping_path, &sap_pool).await?;
    let count = dn_resp.get("value").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0);
    info!("Retrieved {} delivery notes", count);

    println!("\nAll files processed.");
    info!("Processing Time {:?}", start.elapsed());
    Ok(())
}


// basicfun856.exe --dropping-path "C:\Users\BasicFun\Desktop\856\output" --archive-path "C:\Users\BasicFun\Desktop\856\output" --process-id "1" --error-process-data-path "C:\Users\BasicFun\Desktop\856\error"
