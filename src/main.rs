mod order_structure_test;
use order_structure_test::{Orders,Token};

use std::env;
use std::fs;
use clap::Parser;
use serde_json::Value;
use std::time::{Duration, Instant};

use serde::{Deserialize};
use tracing_subscriber::EnvFilter;
use std::error::Error;
use futures::future::join_all;
use tokio::sync::{Semaphore};

use bb8::Pool;
use tracing::{debug, error, info};
use reqwest_middleware::reqwest::{Client, StatusCode};
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::{RetryTransientMiddleware, Retryable, policies::ExponentialBackoff};
use tiberius::{Row, ToSql};

use anyhow::{anyhow, Result};

use roxmltree_to_serde::{Config, NullValue, xml_str_to_json};
use std::{
    fs::File,
    io::Read,
    path::Path,
    path::PathBuf,
    sync::Arc
};

const THREADS: usize = 8;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    input_dir: PathBuf,

    #[arg(long)]
    output_dir: PathBuf,

    #[arg(long)]
    error_dir: PathBuf,
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
fn extract_root_data(
    json: serde_json::value::Value,
) -> Result<Value, Box<dyn std::error::Error>> {
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
fn ensure_array(
    json: &mut Value,
    field: &str,
) {
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
    let json_config = Config::new_with_custom_values(
        true,
        "",
        "txt",
        NullValue::Null,
    );

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
    println!("JSON before deserialization for {}:\n{}", path.display(), serde_json::to_string_pretty(&json).unwrap_or_default());

    // Deserialize into Orders struct
    let orders: Orders = serde_json::from_value(json)
        .map_err(|e| {
            error!("Failed to deserialize Orders from JSON: {e}");
            anyhow!("Failed to deserialize Orders from JSON: {e}")
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
            T0."U_Account_No"
        FROM [@ECSB1_SHIPVIA] T0
        INNER JOIN OSHP T1
            ON T1."TrnspCode" = T0."U_SCAC"
        WHERE
            T0."U_CardCode" = @P1
            AND T0."U_BPM_Ship_VIA" = @P2
    "#;

    info!("Query parameters: card_code='{}', transportation_code='{}'", card_code, transportation_code);
    let rows = send_query(sap_pool, query, &[&card_code, &transportation_code])
        .await
        .map_err(|e| anyhow!("{e}"))?;
    info!("Shipvia rows response: {:#?}", &rows);
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

    info!("TrnspName: {}", trnsp_name);

    let trnsp_code_str = trnsp_code_i16.to_string();
    order.u_transportation_code = Some(trnsp_code_str.clone());
    order.u_ship_scac = Some(trnsp_code_str);
    order.trnsp_code = Some(trnsp_code_i16.into());

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

async fn get_token(client: &ClientWithMiddleware,) -> Result<Token, Box<dyn Error>> {

    let base_url = env::var("BASE_URL").expect("BASE_URL must be set");
    let company = env::var("Company_DB").expect("Company_DB must be set");
    let username = env::var("User_Name").expect("User_Name must be set");
    let password = env::var("Password").expect("Password must be set");
    info!("env cred: {:?}, {:?}, {:?}, {:?}", base_url, company, username, password);

    let login_data = serde_json::json!({
        "CompanyDB": company,
        "Password": password,
        "UserName": username,
    });

    let url = format!("{}/Login", base_url);

    info!("--- Login Attempt ---");
    // Send the POST request with login data
    let response = match client.post(url).json(&login_data).send().await {
        Ok(response) => {
            if response.status().is_success() {
                response
            } else {
                error!(
                    "Failed to get login info, Status Code {}",
                    response.status(),
                );
                return Err("Failed to get login token".into());
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
    let url = format!("{}/Orders", base_url);

    info!("Creating SAP Order");
    info!("URL: {}", url);

    debug!(
        "Payload:\n{}",
        serde_json::to_string_pretty(order)?
    );

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

        error!(
            "SAP order creation failed: {} - {}",
            status,
            body
        );

        return Err(anyhow!(
            "Order creation failed: {} - {}",
            status,
            body
        ));
    }

    // Parse SAP response
    let created: CreateOrderResponse =
        response.json().await?;

    info!(
        "Order created successfully — DocNum: {}, DocEntry: {}",
        created.doc_num,
        created.doc_entry
    );

    Ok(created)
}

async fn update_order_shipping_remarks(
    sap_pool: &Arc<Pool<bb8_tiberius::ConnectionManager>>,
    card_code: &str,
    doc_num: i64,
) -> Result<(), anyhow::Error> {
    info!("Updating shipping remarks for DocNum: {}, CardCode: {}", doc_num, card_code);

    // Query for shipping instructions
    let query = r#"
        SELECT
            CAST(O.TrnspCode AS VARCHAR(20))
            + '|'
            + ISNULL(O.TrnspName, '')
            + ' ||| Ship Window: '
            + CONVERT(VARCHAR(10), GETDATE(), 110)
            + '(Begin)'
            + ' to '
            + CONVERT(VARCHAR(10), GETDATE(), 110)
            + '(End)'
        AS sapShippingInstructions
        FROM OCRD C
        LEFT JOIN OSHP O
            ON C.ShipType = O.TrnspCode
        WHERE C.CardCode = @P1
    "#;

    info!("Fetching shipping instructions for CardCode: {}", card_code);
    let rows = send_query(sap_pool, query, &[&card_code])
        .await
        .map_err(|e| anyhow!("Failed to query shipping instructions: {e}"))?;

    let row = rows
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("No shipping instruction found for card_code {card_code}"))?;

    let shipping_remarks: String = row
        .try_get::<&str, _>("sapShippingInstructions")
        .map_err(|err| anyhow!("Failed to parse sapShippingInstructions: {err}"))?
        .ok_or_else(|| anyhow!("sapShippingInstructions is NULL in the database"))?
        .to_string();

    info!("Fetched shipping remarks: {}", shipping_remarks);

    // Update the order with shipping remarks
    let update_query = r#"
        UPDATE ORDR
        SET U_TBD_SA_Remarks = @P1
        WHERE DocNum = @P2
    "#;

    info!("Updating ORDR table DocNum: {} with remarks", doc_num);
    send_query(sap_pool, update_query, &[&shipping_remarks, &doc_num])
        .await
        .map_err(|e| anyhow!("Failed to update order shipping remarks: {e}"))?;

    info!("Successfully updated shipping remarks for DocNum: {}", doc_num);
    Ok(())
}

async fn process_file(
    input_path: &Path,
    error_path: std::sync::Arc<PathBuf>,
    output_dir: std::sync::Arc<PathBuf>,
    sap_pool: std::sync::Arc<Pool<bb8_tiberius::ConnectionManager>>,
    session_id: &str,
    client: std::sync::Arc<ClientWithMiddleware>,
) -> Result<(), anyhow::Error> {

    info!("Processing: {}", input_path.display());

    // --- Convert XML directly into Orders struct ---
    let mut order_data = match xml_to_json_converter(input_path).await {
        Ok(data) => data,
        Err(e) => {
            error!(
                "Failed to convert {}: {}",
                input_path.display(),
                e
            );

            let dest = error_path.join(
                input_path
                    .file_name()
                    .ok_or(anyhow!("Invalid file name"))?
            );

            let _ = fs::copy(input_path, &dest);

            return Err(anyhow!(
                "Failed to convert XML: {}",
                e
            ));
        }
    };

    // Amazon-specific ship SCAC enrichment only for cardcodes K00008 and D01865.
    if order_data.card_code == "K00008" || order_data.card_code == "D01865" {
        if let Err(err) = enrich_order_with_ship_scac(&sap_pool, &mut order_data).await {
            error!("Failed to enrich order for {}: {err}", input_path.display());
            let dest = error_path.join(
                input_path
                    .file_name()
                    .ok_or(anyhow!("Invalid file name"))?
            );
            let _ = fs::copy(input_path, &dest);
            return Err(anyhow!("Failed to enrich order: {err}"));
        }
    } else {
        info!("Skipping ship SCAC enrichment for non-Amazon card_code={}", order_data.card_code);
    }

    // because ship_to_cod is optional currently
    // if let Some(ship_to_value) = &order_data.ship_to_code {
    //     match get_bp_address(&mut session, ship_to_value, "S") {
    //         Ok(Some(address)) => {
    //             println!("ShipTo AddressName: {}", address);
    //             order_data.ship_to_code = Some(address);

    //             let file_name = input_path.file_name().ok_or("Invalid file name")?;
    //             let output_path = output_dir.join(file_name);

    //             fs::write(&output_path, serde_json::to_string_pretty(&order_data)?)?;
    //             println!("Written to: {}", output_path.display());
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
    //             let output_path = output_dir.join(file_name);

    //             fs::write(&output_path, serde_json::to_string_pretty(&order_data)?)?;
    //             println!("Written to: {}", output_path.display());
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
    match create_order(
        session_id,
        client.as_ref(),
        &order_data,
    ).await {

        Ok(created) => {

            // Update order with shipping remarks
            if let Err(err) = update_order_shipping_remarks(
                &sap_pool,
                &order_data.card_code,
                created.doc_num,
            ).await {
                error!("Failed to update shipping remarks for DocNum {}: {err}", created.doc_num);
                // Continue despite error - still write output
            }

            // --- Build output JSON ---
            let mut output = serde_json::to_value(&order_data)?;

            output["sap_doc_num"] =
                Value::Number(created.doc_num.into());

            output["sap_doc_entry"] =
                Value::Number(created.doc_entry.into());

            let file_name = input_path
                .file_name()
                .ok_or(anyhow!("Invalid file name"))?;

            let output_path = output_dir.join(file_name);

            fs::write(
                &output_path,
                serde_json::to_string_pretty(&output)?
            )?;

            info!(
                "Written output file: {}",
                output_path.display()
            );
        }

        Err(e) => {

            error!(
                "Order creation failed for {}: {}",
                input_path.display(),
                e
            );

            let dest = error_path.join(
                input_path
                    .file_name()
                    .ok_or(anyhow!("Invalid file name"))?
            );

            let _ = fs::copy(input_path, &dest);

            return Err(anyhow!(
                "Failed to create SAP order: {}",
                e
            ));
        }
    }

    Ok(())
}

async fn process_files(
    path: &Path,
    output_dir: std::sync::Arc<PathBuf>,
    error_dir: std::sync::Arc<PathBuf>,
    sap_pool: &Arc<Pool<bb8_tiberius::ConnectionManager>>,
    session_id: &Arc<String>,
    client: &Arc<ClientWithMiddleware>
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
        let out_dir = Arc::clone(&output_dir);
        let err_dir = Arc::clone(&error_dir);

        handles.push(tokio::spawn(async move {
            let _permit = permit;

            match process_file(&path, err_dir, out_dir, sap_pool, session_id.as_str(), client).await {
                Ok(_) => {
                    info!("Processed file: {}", path.display());
                    
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

    // configuring logs
    tracing_subscriber::fmt()
        .with_ansi(false)
        .with_env_filter(EnvFilter::new("info,tiberius=warn"))
        .init();
    dotenv::dotenv().ok();

    // let args: Vec<String> = env::args().collect();

//    if args.len() < 4 {
//         eprintln!("Param Missing");
//         std::process::exit(1);
//     }
//     println!("{:?}, lenght: {}", args, args.len());

//     let input_dir = Path::new(&args[1]);
//     let output_dir = Path::new(&args[2]);
//     let error_dir = Path::new(&args[3]);

    let args = Args::parse();
    let input_dir = Arc::new(args.input_dir);
    let output_dir = Arc::new(args.output_dir);
    let error_dir = Arc::new(args.error_dir);

    info!("Input:  {}", input_dir.display());
    info!("Output: {}", output_dir.display());
    info!("Error:  {}", error_dir.display());

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
    let retry_policy =
        ExponentialBackoff::builder()
        .retry_bounds(Duration::from_millis(100), Duration::from_secs(30))
        .build_with_total_retry_duration(Duration::from_secs(2 * 60));

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
        Arc::clone(&output_dir),
        Arc::clone(&error_dir),
        &sap_pool,
        &session_id,
        &client,
    ).await?;

    println!("\nAll files processed.");
    info!("Processing Time {:?}", start.elapsed());
    Ok(())
}

// /Users/noor/Public/Ecom/input_files /Users/noor/Public/Ecom/output_files /Users/noor/Public/Ecom/error_files
// basic_fun.exe C:/Users/BasicFun/Desktop/test/input/ C:/Users/BasicFun/Desktop/test/output/ C:/Users/BasicFun/Desktop/test/error/  
// basic_fun.exe --input-dir "C:\Users\BasicFun\Desktop\test\input" --output-dir "C:\Users\BasicFun\Desktop\test\output" --error-dir "C:\Users\BasicFun\Desktop\test\error"


// 945 path: 
// basic_fun_945.exe --files-pickup-path C:\Users\BasicFun\Desktop\945\input --process-id 1234 --process-data-path C:\Users\BasicFun\Desktop\945\output --prism-path C:\Users\BasicFun\Desktop\945\error