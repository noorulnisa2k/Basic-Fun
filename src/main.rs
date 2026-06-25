mod order_structure_test;
use order_structure_test::{Orders, Token};
mod prism_structure;
use prism_structure::{Agency, Output};

use clap::Parser;
use serde_json::Value;
use std::env;
use std::fs;
use std::time::{Duration, Instant};

use futures::future::join_all;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::HashMap;
use std::error::Error;
use tokio::sync::Semaphore;
use tracing_subscriber::EnvFilter;

use bb8::Pool;
use reqwest_middleware::reqwest::{Client, StatusCode};
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware, Retryable};
use tiberius::{Row, ToSql};
use tracing::{debug, error, info};

use anyhow::{anyhow, Result};

use roxmltree_to_serde::{xml_str_to_json, Config, NullValue};
use std::{
    fs::{File, OpenOptions},
    io::Read,
    path::Path,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tracing_subscriber::fmt::MakeWriter;

const THREADS: usize = 8;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    input_dir: PathBuf,

    #[arg(long)]
    archive_dir: PathBuf,

    #[arg(long)]
    error_dir: PathBuf,

    #[arg(long)]
    logs_dir: PathBuf,

    #[arg(long)]
    prism_path: PathBuf,

    #[arg(long)]
    process_data_path: PathBuf,

    #[arg(short, long)]
    process_id: String,
}

struct LogFileWriter {
    file: Mutex<File>,
}

impl<'a> MakeWriter<'a> for LogFileWriter {
    type Writer = File;

    fn make_writer(&'a self) -> Self::Writer {
        self.file.lock().unwrap().try_clone().unwrap()
    }
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

// #[derive(Deserialize, Debug)]
// struct BPAddressEntry {
//     #[serde(rename = "BusinessPartners/BPAddresses")]
//     bp_addresses: BPAddressFields,
// }

// #[derive(Deserialize, Debug)]
// struct BPAddressFields {
//     #[serde(rename = "AddressName")]
//     address_name: String,
// }

// #[derive(Deserialize, Debug)]
// struct BPAddressResponse {
//     value: Vec<BPAddressEntry>,
// }

#[derive(Deserialize, Debug)]
struct CreateOrderResponse {
    #[serde(rename = "DocNum")]
    doc_num: i64,
    #[serde(rename = "DocEntry")]
    doc_entry: i64,
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

// Downcasts the given err source into T.
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

fn classify_io_error(error: &std::io::Error) -> Retryable {
    match error.kind() {
        std::io::ErrorKind::ConnectionReset | std::io::ErrorKind::ConnectionAborted => {
            Retryable::Transient
        }
        _ => Retryable::Fatal,
    }
}

fn fetch_prism_data(process_data_path: &Path) -> Result<HashMap<String, String>> {
    let mut file_map = HashMap::new();
    for entry in fs::read_dir(process_data_path)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            let contents = fs::read_to_string(&path)?;
            let output: Output = match quick_xml::de::from_str(&contents) {
                Ok(o) => o,
                Err(err) => {
                    error!("Failed to parse prism data {}: {err}", path.display());
                    continue;
                }
            };
            if let Some(erp) = output.erp_file_name {
                let name = Path::new(&erp.into_owned())
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .to_string();
                file_map.insert(name, path.to_string_lossy().to_string());
            }
            if let Some(edi) = output.edi_file_name {
                let name = Path::new(&edi.into_owned())
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .to_string();
                file_map.insert(name, path.to_string_lossy().to_string());
            }
        }
    }
    info!("Loaded {} Prism process data files", file_map.len());
    Ok(file_map)
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
) -> Result<Vec<Row>, Box<dyn std::error::Error>> {
    debug!("Reaching send_query function with query {query_str}");
    info!("Sending SQL query:\n{}", query_str);

    let mut conn = pool
        .get()
        .await
        .map_err(|err| anyhow!("Failed to get Pool Connection in send_query function: {err}"))?;

    let stream = conn.query(query_str, parameters).await.map_err(|err| {
        error!("SQL Error Response for Query: {query_str}, {err}");
        Box::new(err) as Box<dyn std::error::Error>
    })?;

    let rows = stream.into_first_result().await.map_err(|err| {
        error!("Failed to fetch result for query {query_str}: {err}");
        Box::new(err) as Box<dyn std::error::Error>
    })?;

    debug!("Query {query_str} returning rows count {}", rows.len());
    info!("Query response:\n{:#?}", rows);
    Ok(rows)
}

// Extract root node from JSON
fn extract_root_data(json: serde_json::value::Value) -> Result<Value, Box<dyn std::error::Error>> {
    // Check if the root is an object
    if let Value::Object(root) = json {
        // Take the first key-value pair (if any)
        if let Some((_key, inner_content)) = root.into_iter().next() {
            // Return inner content
            Ok(inner_content)
        } else {
            Err("Root object is empty".into())
        }
    } else {
        Err("Root is not a JSON object".into())
    }
}

// DocumentLines into list of it is not already
fn ensure_array(json: &mut Value, field: &str) {
    if let Some(value) = json.get_mut(field) {
        // Already array
        if value.is_array() {
            return;
        }

        // Convert object → array
        let single = value.take();

        *value = Value::Array(vec![single]);
    }
}

// convert XML to json
async fn xml_to_json_converter(path: &Path) -> Result<Orders> {
    // Open XML file
    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    debug!("XML Content:\n{}\n", contents);

    // XML → JSON Config
    let json_config = Config::new_with_custom_values(true, "", "txt", NullValue::Null);

    // Convert XML → JSON
    let json_struct = match xml_str_to_json(&contents, &json_config) {
        Ok(json) => json,
        Err(err) => {
            error!("Failed to convert XML to JSON: {err}");
            return Err(anyhow!("Failed to convert XML to JSON: {err}"));
        }
    };

    // Extract root data
    let mut json = match extract_root_data(json_struct) {
        Ok(json) => json,
        Err(err) => panic!("Failed to Extract Root Data from JSON, {err}"),
    };

    // Ensure DocumentLines is an array
    ensure_array(&mut json, "DocumentLines");

    // Print the JSON before deserialization so the exact payload can be inspected.
    println!(
        "JSON before deserialization for {}:\n{}",
        path.display(),
        serde_json::to_string_pretty(&json).unwrap_or_default()
    );

    // Deserialize into Orders struct
    let orders: Orders = serde_path_to_error::deserialize(json).map_err(|e| {
        error!(
            "Failed to deserialize Orders from JSON: {} at {}",
            e.inner(),
            e.path()
        );
        anyhow!(
            "Failed to deserialize Orders from JSON: {} at {}",
            e.inner(),
            e.path()
        )
    })?;

    Ok(orders)
}

async fn enrich_order_with_ship_scac(
    sap_pool: &Arc<Pool<bb8_tiberius::ConnectionManager>>,
    order: &mut Orders,
) -> Result<(), anyhow::Error> {
    let card_code = order.card_code.clone();
    let transportation_code = order
        .u_transportation_code
        .as_ref()
        .ok_or_else(|| anyhow!("Missing TransportationCode for order"))?
        .to_string();

    let query = r#"
        SELECT
            T1."TrnspCode",
            T1."TrnspName",
            T1."U_SCAC",
            T0."U_Account_No",
            T0."U_Account_Zipcode"
        FROM [@ECSB1_SHIPVIA] T0
        INNER JOIN OSHP T1
            ON T1."TrnspCode" = T0."U_SCAC"
        WHERE
            T0."U_CardCode" = @P1
            AND T0."U_BPM_Ship_VIA" = @P2
    "#;

    info!(
        "Query parameters: card_code='{}', transportation_code='{}'",
        card_code, transportation_code
    );
    let rows = send_query(sap_pool, query, &[&card_code, &transportation_code])
        .await
        .map_err(|e| anyhow!("{e}"))?;
    let row = rows
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("No matching shipvia row for card_code {card_code}, transportation_code {transportation_code}"))?;

    let trnsp_code_i16: i16 = row
        .try_get::<i16, _>("TrnspCode")
        .map_err(|err| anyhow!("Failed to parse TrnspCode: {err}"))?
        .ok_or_else(|| anyhow!("TrnspCode is NULL in the database"))?;

    let trnsp_name: String = row
        .try_get::<&str, _>("TrnspName")
        .map_err(|err| anyhow!("Failed to parse TrnspName: {err}"))?
        .ok_or_else(|| anyhow!("TrnspName is NULL in the database"))?
        .to_string();
    let scac_u: String = row
        .try_get::<&str, _>("U_SCAC")
        .map_err(|err| anyhow!("Failed to parse U_SCAC: {err}"))?
        .ok_or_else(|| anyhow!("U_SCAC is NULL in the database"))?
        .to_string();
    let ship_via_acct: String = row
        .try_get::<&str, _>("U_Account_No")
        .map_err(|err| anyhow!("Failed to parse U_Account_No: {err}"))?
        .ok_or_else(|| anyhow!("U_Account_No is NULL in the database"))?
        .to_string();

    let account_zip: String = row
        .try_get::<&str, _>("U_Account_Zipcode")
        .map_err(|err| anyhow!("Failed to parse U_Account_Zipcode: {err}"))?
        .ok_or_else(|| anyhow!("U_Account_Zipcode is NULL in the database"))?
        .to_string();

    info!("TrnspName: {}", trnsp_name);

    let trnsp_code_str = trnsp_code_i16.to_string();
    order.u_transportation_code = Some(trnsp_code_str.clone());
    order.u_ship_scac = Some(scac_u);
    order.u_ship_scac = Some(trnsp_code_str);
    order.trnsp_code = Some(trnsp_code_i16.into());
    // order.u_scac = Some(scac_u);
    order.u_ship_via_acct = Some(ship_via_acct);
    order.u_account_zip = Some(account_zip.clone());

    // Extract U_SCAC and U_Account_No from the query result and populate the order
    let u_scac_opt: Option<String> = match row.try_get::<&str, _>("U_SCAC") {
        Ok(Some(s)) => Some(s.to_string()),
        Ok(None) => None,
        Err(err) => return Err(anyhow!("Failed to parse U_SCAC: {err}")),
    };

    let u_account_no_opt: Option<String> = match row.try_get::<&str, _>("U_Account_No") {
        Ok(Some(s)) => Some(s.to_string()),
        Ok(None) => None,
        Err(err) => return Err(anyhow!("Failed to parse U_Account_No: {err}")),
    };

    if let Some(u_scac_val) = u_scac_opt {
        order.u_scac = Some(u_scac_val);
    }

    if let Some(acc_no) = u_account_no_opt {
        order.u_ship_via_acct = Some(acc_no);
    }

    Ok(())
}

async fn billing_type_check(
    sap_pool: &Arc<Pool<bb8_tiberius::ConnectionManager>>,
    order: &mut Orders,
) -> Result<(), anyhow::Error> {
    let card_code = order.card_code.clone();
    let query = r#"
        SELECT
            U_PayType
        FROM OCRD
        WHERE
            CardCode = @P1
    "#;

    info!("Query parameters: card_code='{}'", card_code);
    let rows = send_query(sap_pool, query, &[&card_code])
        .await
        .map_err(|e| anyhow!("{e}"))?;
    let row = rows
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("No matching shipvia row for card_code {card_code}"))?;

    let bill_type: String = row
        .try_get::<&str, _>("U_PayType")
        .map_err(|err| anyhow!("Failed to parse U_PayType: {err}"))?
        .ok_or_else(|| anyhow!("U_PayType is NULL in the database"))?
        .to_string();

    info!("Bill Type: {}", bill_type);

    order.u_billing_type = Some(bill_type);

    Ok(())
}

async fn get_token(client: &ClientWithMiddleware) -> Result<Token, Box<dyn Error>> {
    let base_url = env::var("BASE_URL").expect("BASE_URL must be set");
    let company = env::var("Company_DB").expect("Company_DB must be set");
    let username = env::var("User_Name").expect("User_Name must be set");
    let password = env::var("Password").expect("Password must be set");
    info!(
        "env cred: {:?}, {:?}, {:?}, {:?}",
        base_url, company, username, password
    );

    let login_data = serde_json::json!({
        "CompanyDB": company,
        "Password": password,
        "UserName": username,
    });

    let url = format!("{}/b1s/v1/Login", base_url);

    info!("--- Login Attempt ---");

    info!("Login URL: {}", url);
    info!("Login payload: {}", login_data);

    // Send the POST request with login data
    let response = match client.post(url).json(&login_data).send().await {
        Ok(response) => {
            if response.status().is_success() {
                response
            } else {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                error!(
                    "Failed to get login info, Status Code {}. Response body: {}",
                    status, body,
                );
                return Err(
                    format!("Failed to get login token. Status: {status}, Body: {body}").into(),
                );
            }
        }
        Err(err) => {
            error!("Failed to get token: {err:#}");
            return Err(format!("Failed to get token: {err:#}").into());
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

// fn get_bp_address(
//     session: &mut SapSession,
//     data: &str, a_type: &str,
// ) -> Result<Option<String>, Box<dyn Error>> {
//     // Proactively refresh if expired before making the call
//     ensure_session(session)?;

//     println!("--------inside get_bp_address---------");

//     let parts: Vec<&str> = data.split(',').collect();
//     let base_url = env::var("BASE_URL")?;

//     let url = format!(
//         "{}/$crossjoin(BusinessPartners,BusinessPartners/BPAddresses)\
//         ?$expand=BusinessPartners($select=CardType,CardCode),\
//         BusinessPartners/BPAddresses($select=AddressName,AddressType,GlobalLocationNumber)\
//         &$filter=BusinessPartners/CardCode eq BusinessPartners/BPAddresses/BPCode \
//         and BusinessPartners/CardCode eq '{}' \
//         and BusinessPartners/BPAddresses/GlobalLocationNumber eq '{}' \
//         and BusinessPartners/CardType eq '{}' \
//         and BusinessPartners/BPAddresses/AddressType eq '{}'",
//         base_url, parts[3], parts[1], parts[0], a_type
//     );

//     println!("Request URL:\n{}", url);

//     let response = session
//         .client
//         .get(&url)
//         .header("Cookie", &session.cookies)
//         .send()?;

//     let response = if response.status() == 401 {
//         println!("Received 401. Re-authenticating and retrying...");
//         *session = sap_login()?;

//         let retry = session
//             .client
//             .get(&url)
//             .header("Cookie", &session.cookies)
//             .send()?;

//         if !retry.status().is_success() {
//             let status = retry.status();
//             let body = retry.text().unwrap_or_default();
//             return Err(format!("Retry failed after re-auth: {} - {}", status, body).into());
//         }
//         retry
//     } else {
//     if !response.status().is_success() {
//         let status = response.status();
//         let body = response.text().unwrap_or_default();
//         return Err(format!("Request failed: {} - {}", status, body).into());
//     }
//         response
//     };

//     let body = response.text()?;
//     println!("bp_address Response: {}", body);

//     let parsed: BPAddressResponse = serde_json::from_str(&body)?;
//     // let parsed: BPAddressResponse = response.json()?;

//     let ship_to_code = parsed
//         .value
//         .into_iter()
//         .next()
//         .map(|entry| entry.bp_addresses.address_name);

//     Ok(ship_to_code)
// }
async fn create_order(
    session_id: &str,
    client: &ClientWithMiddleware,
    order: &Orders,
) -> Result<CreateOrderResponse, anyhow::Error> {
    info!("-------- inside create_order --------");

    let base_url = env::var("BASE_URL")?;
    let url = format!("{}/b1s/v1/Orders", base_url);

    info!("Creating SAP Order");
    info!("URL: {}", url);

    let payload = serde_json::to_string_pretty(order)?;
    info!("Payload:\n{}", payload);

    let response = client
        .post(&url)
        .header("Cookie", session_id)
        .json(order)
        .send()
        .await?;

    // Handle failed response
    if !response.status().is_success() {
        let status = response.status();

        let body = match response.text().await {
            Ok(text) => text,
            Err(_) => "Unable to read response body".to_string(),
        };

        error!("SAP order creation failed: {} - {}", status, body);

        return Err(anyhow!("Order creation failed: {} - {}", status, body));
    }

    // Parse SAP response
    let created: CreateOrderResponse = response.json().await?;

    info!(
        "Order created successfully — DocNum: {}, DocEntry: {}",
        created.doc_num, created.doc_entry
    );

    Ok(created)
}

async fn update_order_shipping_remarks(
    sap_pool: &Arc<Pool<bb8_tiberius::ConnectionManager>>,
    card_code: &str,
    doc_num: i64,
    begin_window: &Option<String>,
    end_window: &Option<String>,
    account_no: &Option<String>,
) -> Result<(), anyhow::Error> {
    info!(
        "Updating shipping remarks for DocNum: {}, CardCode: {}",
        doc_num, card_code
    );

    // Query for shipping instructions
    // let query = r#"
    //     SELECT
    //         O."TrnspCode", O."TrnspName", T0."U_Account_No", T0."U_Account_Zipcode"
    //     FROM OSHP O
    //     LEFT JOIN [@ECSB1_SHIPVIA] T0
    //         ON T0."U_SCAC" = O.TrnspCode
    //     WHERE T0.U_CardCode = @P1
    // "#;

    let query = r#"
        SELECT
            O.TrnspCode, O.TrnspName
        FROM OSHP O
        INNER JOIN OCRD T0
            ON T0.ShipType = O.TrnspCode
        WHERE
            T0."CardCode" = @P1"#;

    // let account_query ="SELECT U_BeginWindowDate, U_EndWindowDate FROM ORDR WHERE DocNum = @P1";

    let mut query_results = String::new();

    info!("Fetching shipping instructions for CardCode: {}", card_code);
    let query_rows = send_query(sap_pool, query, &[&card_code])
        .await
        .map_err(|e| anyhow!("Failed to query shipping instructions: {e}"))?;
    // info!("Shipping instructions query returned {:#?}", &query_rows);
    // let ordr_rows = send_query(sap_pool, account_query, &[&doc_num])
    //     .await
    //     .map_err(|e| anyhow!("Failed to query order details: {e}"))?;
    // info!("Order details query returned {:#?}", &ordr_rows);
    match query_rows.first() {
        Some(row) => {
            let trnsp_code: Option<i16> = row.try_get("TrnspCode").unwrap_or(None);
            let trnsp_name: Option<&str> = row.try_get("TrnspName").unwrap_or(None);
            // let account_no: Option<&str> = row.try_get("U_Account_No").unwrap_or(None);
            // let account_zip: Option<&str> = row.try_get("U_Account_Zipcode").unwrap_or(None);
            query_results = format!(
                "{}|{}|Account# {}",
                trnsp_code.map(|v| v.to_string()).unwrap_or_default(),
                trnsp_name.as_deref().unwrap_or(""),
                account_no.as_deref().unwrap_or("")
            );
            // info!("Shipping Instructions - TrnspCode: {:?}, TrnspName: {:?}, AccountNo: {:?}, AccountZip: {:?}", trnsp_code, trnsp_name, account_no, account_zip);
        }
        None => {
            error!("No shipping instructions found for CardCode: {}", card_code);
            // return Err(anyhow!("No shipping instructions found for CardCode: {}", card_code));
        }
    }
    query_results = format!(
        "{}| Ship Window: {}(Begin) to {}(End)",
        query_results,
        begin_window.as_deref().unwrap_or(""),
        end_window.as_deref().unwrap_or("")
    );
    info!("{}", query_results);

    // match ordr_rows.first(){
    //     Some(row) =>{
    //         let shipping_begin : Option<&str> = row.try_get("U_BeginWindowDate").unwrap_or(None);
    //         let shipping_end : Option<&str> = row.try_get("U_EndWindowDate").unwrap_or(None);
    //         query_results = format!("{:?}| Ship Window: {:?}(Begin) to {:?}(End)", query_results, shipping_begin, shipping_end);
    //     }
    //     None => {
    //         error!("No shipping instructions found for CardCode: {}", card_code);
    //         // return Err(anyhow!("No shipping instructions found for CardCode: {}", card_code));
    //     }
    // }
    // let row = rows
    //     .into_iter()
    //     .next()
    //     .ok_or_else(|| anyhow!("No shipping instruction found for card_code {card_code}"))?;

    // let shipping_remarks: String = row
    //     .try_get::<&str, _>("sapShippingInstructions")
    //     .map_err(|err| anyhow!("Failed to parse sapShippingInstructions: {err}"))?
    //     .ok_or_else(|| anyhow!("sapShippingInstructions is NULL in the database"))?
    //     .to_string();

    // info!("Fetched shipping remarks: {}", shipping_remarks);

    // Update the order with shipping remarks
    let update_query = r#"
        UPDATE ORDR
        SET U_TBD_SA_Remarks = @P1
        WHERE DocNum = @P2
    "#;

    info!("Updating ORDER table DocNum: {} with remarks", doc_num);
    send_query(sap_pool, update_query, &[&query_results, &doc_num])
        .await
        .map_err(|e| anyhow!("Failed to update order shipping remarks: {e}"))?;

    info!(
        "Successfully updated shipping remarks for DocNum: {}",
        doc_num
    );
    Ok(())
}

async fn process_file(
    input_path: &Path,
    error_path: std::sync::Arc<PathBuf>,
    archive_dir: std::sync::Arc<PathBuf>,
    sap_pool: std::sync::Arc<Pool<bb8_tiberius::ConnectionManager>>,
    session_id: &str,
    client: std::sync::Arc<ClientWithMiddleware>,
) -> Result<(), anyhow::Error> {
    info!("Processing: {}", input_path.display());

    // --- Convert XML directly into Orders struct ---
    let mut order_data = match xml_to_json_converter(input_path).await {
        Ok(data) => data,
        Err(e) => {
            error!("Failed to convert {}: {}", input_path.display(), e);

            let dest = error_path.join(input_path.file_name().ok_or(anyhow!("Invalid file name"))?);

            let _ = fs::copy(input_path, &dest);

            return Err(anyhow!("Failed to convert XML: {}", e));
        }
    };
    if order_data.card_code.is_empty() {
        if let Err(err) = billing_type_check(&sap_pool, &mut order_data).await {
            error!("Failed to enrich order for {}: {err}", input_path.display());
            let dest = error_path.join(input_path.file_name().ok_or(anyhow!("Invalid file name"))?);
            let _ = fs::copy(input_path, &dest);
            return Err(anyhow!("Order billing type check failed: {err}"));
        }
    }

    // Amazon-specific ship SCAC enrichment only for cardcodes K00008 and D01865.
    if order_data.card_code == "K00008" || order_data.card_code == "D01865" {
        if let Err(err) = enrich_order_with_ship_scac(&sap_pool, &mut order_data).await {
            error!("Failed to enrich order for {}: {err}", input_path.display());
            let dest = error_path.join(input_path.file_name().ok_or(anyhow!("Invalid file name"))?);
            let _ = fs::copy(input_path, &dest);
            return Err(anyhow!("Failed to enrich order: {err}"));
        }
    } else {
        info!(
            "Skipping ship SCAC enrichment for non-Amazon card_code={}",
            order_data.card_code
        );
    }

    // because ship_to_cod is optional currently
    // if let Some(ship_to_value) = &order_data.ship_to_code {
    //     match get_bp_address(&mut session, ship_to_value, "S") {
    //         Ok(Some(address)) => {
    //             println!("ShipTo AddressName: {}", address);
    //             order_data.ship_to_code = Some(address);

    //             let file_name = input_path.file_name().ok_or("Invalid file name")?;
    //             let archive_path = archive_dir.join(file_name);

    //             fs::write(&archive_path, serde_json::to_string_pretty(&order_data)?)?;
    //             println!("Written to: {}", archive_path.display());
    //         }
    //         Ok(None) => {
    //             eprintln!("No ShipTo address found for ShipToCode: {}", ship_to_value);
    //             let dest = error_dir.join(input_path.file_name().unwrap());
    //             let _ = fs::copy(input_path, &dest);
    //         }
    //         Err(e) => {
    //             eprintln!("API error for {}: {}", file_path, e);
    //             let dest = error_dir.join(input_path.file_name().unwrap());
    //             let _ = fs::copy(input_path, &dest);
    //         }
    //     }
    // }

    // if let Some(pay_to_value) = &order_data.pay_to_code {
    //     match get_bp_address(&mut session, pay_to_value, "B") {
    //         Ok(Some(address)) => {
    //             println!("PayTo AddressName: {}", address);
    //             order_data.pay_to_code = Some(address);

    //             let file_name = input_path.file_name().ok_or("Invalid file name")?;
    //             let archive_path = archive_dir.join(file_name);

    //             fs::write(&archive_path, serde_json::to_string_pretty(&order_data)?)?;
    //             println!("Written to: {}", archive_path.display());
    //         }
    //         Ok(None) => {
    //             eprintln!("No PayTo address found for PayToCode: {}", pay_to_value);
    //             let dest = error_dir.join(input_path.file_name().unwrap());
    //             let _ = fs::copy(input_path, &dest);
    //         }
    //         Err(e) => {
    //             eprintln!("API error for {}: {}", file_path, e);
    //             let dest = error_dir.join(input_path.file_name().unwrap());
    //             let _ = fs::copy(input_path, &dest);
    //         }
    //     }
    // }

    // --- Create order in SAP ---
    match create_order(session_id, client.as_ref(), &order_data).await {
        Ok(created) => {
            // Update order with shipping remarks
            if let Err(err) = update_order_shipping_remarks(
                &sap_pool,
                &order_data.card_code,
                created.doc_num,
                &order_data.u_begin_window_date,
                &order_data.u_end_window_date,
                &order_data.u_ship_via_acct,
            )
            .await
            {
                error!(
                    "Failed to update shipping remarks for DocNum {}: {err}",
                    created.doc_num
                );
                // Continue despite error - still write output
            }

            // --- Build output JSON ---
            let mut output = serde_json::to_value(&order_data)?;

            output["sap_doc_num"] = Value::Number(created.doc_num.into());

            output["sap_doc_entry"] = Value::Number(created.doc_entry.into());

            let file_name = input_path.file_name().ok_or(anyhow!("Invalid file name"))?;

            let archive_path = archive_dir.join(file_name);

            fs::write(&archive_path, serde_json::to_string_pretty(&output)?)?;

            info!("Written archive file: {}", archive_path.display());
        }

        Err(e) => {
            error!("Order creation failed for {}: {}", input_path.display(), e);

            let dest = error_path.join(input_path.file_name().ok_or(anyhow!("Invalid file name"))?);

            let _ = fs::copy(input_path, &dest);

            return Err(anyhow!("Failed to create SAP order: {}", e));
        }
    }

    Ok(())
}

async fn process_files(
    path: &Path,
    archive_dir: std::sync::Arc<PathBuf>,
    error_dir: std::sync::Arc<PathBuf>,
    sap_pool: &Arc<Pool<bb8_tiberius::ConnectionManager>>,
    session_id: &Arc<String>,
    client: &Arc<ClientWithMiddleware>,
    // file_hashmap: &Arc<HashMap<String, String>>,
    prism_path: &Arc<PathBuf>,
) -> Result<(), anyhow::Error> {
    let semaphore = Arc::new(Semaphore::new(THREADS));
    let mut handles = Vec::new();

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
        let archive_dir = Arc::clone(&archive_dir);
        let err_dir = Arc::clone(&error_dir);
        // let file_hashmap = Arc::clone(file_hashmap);
        let prism_path = Arc::clone(prism_path);

        handles.push(tokio::spawn(async move {
            let _permit = permit;
            let input_file_name = path.file_name().unwrap().to_string_lossy().to_string();
            // let prism_file = file_hashmap.get(&input_file_name).cloned();

            match process_file(
                &path,
                err_dir,
                archive_dir,
                sap_pool,
                session_id.as_str(),
                client,
            )
            .await
            {
                Ok(_) => {
                    info!("Processed file: {}", path.display());

                    // if let Some(prism_file) = &prism_file {
                    //     let mut prism_dest = PathBuf::from(prism_path.as_ref());
                    //     prism_dest.push(Path::new(prism_file).file_name().unwrap());
                    //     if let Err(err) = tokio::fs::rename(prism_file, &prism_dest).await {
                    //         error!("Failed to move prism file {prism_file}: {err}");
                    //     }
                    // }

                    match tokio::fs::remove_file(&path).await {
                        Ok(_) => {
                            info!("Removed {}", path.display());
                        }
                        Err(err) => {
                            error!("Failed to remove 850 XML {}: {err}", path.display());
                            return Err(anyhow!(
                                "Failed to remove 850 XML {}: {err}",
                                path.display()
                            ));
                        }
                    };
                    Ok(())
                }
                Err(e) => {
                    // if let Some(prism_file) = &prism_file {
                    //     let contents = match std::fs::read_to_string(prism_file) {
                    //         Ok(c) => c,
                    //         Err(err) => {
                    //             error!("Failed to read prism file {prism_file}: {err}");
                    //             String::new()
                    //         }
                    //     };
                    //     if !contents.is_empty() {
                    //         if let Ok(mut process_data) = quick_xml::de::from_str::<Output>(&contents) {
                    //             process_data.error_type = Some(Cow::Borrowed("ERP"));
                    //             process_data.agency = Agency::ERROR;
                    //             process_data.error_description = Some(Cow::from(e.to_string()));
                    //             process_data.status = Cow::Borrowed("ERROR");
                    //             process_data.plant_name = Some(Cow::Borrowed("BasicFun"));
                    //             let mut xml_string = String::new();
                    //             if let Ok(ser) = quick_xml::se::Serializer::with_root(&mut xml_string, Some("Output")) {
                    //                 let _ = process_data.serialize(ser);
                    //             }
                    //             let mut prism_dest = PathBuf::from(prism_path.as_ref());
                    //             prism_dest.push(Path::new(prism_file).file_name().unwrap());
                    //             if let Err(err) = tokio::fs::write(&prism_dest, &xml_string).await {
                    //                 error!("Failed to write prism error file: {err}");
                    //             }
                    //         }
                    //     }
                    // }

                    match tokio::fs::remove_file(&path).await {
                        Ok(_) => {
                            info!("Removed {}", path.display());
                        }
                        Err(err) => {
                            error!("Failed to remove 850 XML {}: {err}", path.display());
                            return Err(anyhow!(
                                "Failed to remove 850 XML {}: {err}",
                                path.display()
                            ));
                        }
                    };
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv::dotenv().ok();

    let args = Args::parse();

    // Create logs directory and set up log file
    fs::create_dir_all(&args.logs_dir)?;
    let log_path = args.logs_dir.join(format!(
        "logs850_{}.log",
        chrono::Local::now().format("%Y-%m-%d")
    ));
    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;
    let log_writer = LogFileWriter {
        file: Mutex::new(log_file),
    };

    // configuring logs
    tracing_subscriber::fmt()
        .with_writer(log_writer)
        .with_ansi(false)
        .with_env_filter(EnvFilter::new("info,tiberius=warn"))
        .init();

    let input_dir = Arc::new(args.input_dir);
    let archive_dir = Arc::new(args.archive_dir);
    let error_dir = Arc::new(args.error_dir);

    info!("Input:   {}", input_dir.display());
    info!("Archive: {}", archive_dir.display());
    info!("Error:   {}", error_dir.display());
    info!(
        "Logs:    {} ({})",
        args.logs_dir.display(),
        log_path.display()
    );

    fs::create_dir_all(&args.prism_path)?;
    let prism_path = Arc::new(args.prism_path);
    let process_data_path = args.process_data_path;

    info!("Prism:   {}", prism_path.display());
    info!("Process Data: {}", process_data_path.display());

    // let file_hashmap = Arc::new(fetch_prism_data(&process_data_path)?);

    let sap_pool = Arc::new(setup_db_pool("SAP_DB_CONN").await?);

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
    let retry_policy = ExponentialBackoff::builder()
        .retry_bounds(Duration::from_millis(100), Duration::from_secs(30))
        .build_with_total_retry_duration(Duration::from_secs(2));

    let ret_s =
        RetryTransientMiddleware::new_with_policy_and_strategy(retry_policy, FullRetryableStrategy);

    // Wrap with retry middleware
    let client = ClientBuilder::new(base_client).with(ret_s).build();
    let client = Arc::new(client);

    let start = Instant::now();
    let token = get_token(client.as_ref()).await?;
    let session_id = Arc::new(["B1SESSION=", token.session_id.as_str()].concat());

    // Process files in input dir
    process_files(
        input_dir.as_path(),
        Arc::clone(&archive_dir),
        Arc::clone(&error_dir),
        &sap_pool,
        &session_id,
        &client,
        // &file_hashmap,
        &prism_path,
    )
    .await?;

    println!("\nAll files processed.");
    info!("Processing Time {:?}", start.elapsed());
    Ok(())
}

// /Users/noor/Public/Ecom/input_files /Users/noor/Public/Ecom/archive_files /Users/noor/Public/Ecom/error_files
// basic_fun.exe C:/Users/BasicFun/Desktop/test/input/ C:/Users/BasicFun/Desktop/test/archive/ C:/Users/BasicFun/Desktop/test/error/
// basic_fun.exe --input-dir "C:\Users\BasicFun\Desktop\test\input" --archive-dir "C:\Users\BasicFun\Desktop\test\output" --error-dir "C:\Users\BasicFun\Desktop\test\error" --logs-dir "C:\Users\BasicFun\Desktop\test\logs" --prism-path "C:\Users\BasicFun\Desktop\test\output" --process-data-path "C:\Users\BasicFun\Desktop\test\output" -p "12345"
