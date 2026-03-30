mod order_structure;
mod prism_structure;

use bb8::Pool;
use chrono::Local;
use dotenv::dotenv;
// use order_structure::AddressExtension;
use anyhow::{anyhow, Context, Result};
use clap::Parser;
use futures::future::join_all;
use order_structure::{Orders, Root, Token};
use quick_xml::se::to_string_with_root;
// use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE, COOKIE};
// use reqwest::{Client, Method};
use bytes::Bytes;
use reqwest_middleware::reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE, COOKIE};
use reqwest_middleware::reqwest::{Client, Method, StatusCode};
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware, Retryable};
use rust_decimal::Decimal;
use serde::de::IntoDeserializer;
use serde_json::json;
use serde_json::Value;
use std::borrow::Cow;
use std::env;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tiberius::{Row, ToSql};
use tokio::time::{Duration, Instant};
use tokio::{fs::write, sync::Semaphore, task};
use tracing::{debug, error, info, warn};

use crate::prism_structure::Agency;
use tracing_subscriber::EnvFilter;

// use std::process::CommandArgs;

/*
#[derive(Debug, Clone)]
enum SqlType {
    // Number(i32),
    String(String),
}
*/

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

const BASE_URL: &str = "https://BFAZWSAP01.corp.basicfun.com:50000";
const THREADS: usize = 8;
// const SEND_PATCH_ATTEMPTS: usize = 3;

pub struct FullRetryableStrategy;
impl reqwest_retry::RetryableStrategy for FullRetryableStrategy {
    fn handle(
        &self,
        res: &Result<reqwest::Response, reqwest_middleware::Error>,
    ) -> Option<Retryable> {
        match res {
            Ok(success) => default_on_request_success(success),
            Err(error) => default_on_request_failure(error),
        }
    }
}

/*
pub fn default_on_request_success(
    success: &reqwest_middleware::reqwest::Response,
) -> Option<Retryable> {
    let status = success.status();
    if status.is_server_error() {
        Some(Retryable::Transient)
    } else if status.is_client_error()
        // && status != StatusCode::BAD_REQUEST
        && status != StatusCode::REQUEST_TIMEOUT
        && status != StatusCode::TOO_MANY_REQUESTS
    {
        Some(Retryable::Fatal)
    } else if status.is_success() {
        None
    } else if status == StatusCode::REQUEST_TIMEOUT
        // || status == StatusCode::BAD_REQUEST
        || status == StatusCode::TOO_MANY_REQUESTS
    {
        Some(Retryable::Transient)
    } else {
        Some(Retryable::Fatal)
    }
}
*/

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

// TODO: UPDATE IF LET TO ERROR IF GET IS NOT POPULATED

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    tracing_subscriber::fmt()
        .with_ansi(false)
        .with_env_filter(EnvFilter::new("info,tiberius=warn"))
        .init();

    // Clap Args
    let mut args = Args::parse();
    let dropping_path = Arc::new(args.dropping_path);
    let now = Local::now();
    args.archive_path.push(now.format("%Y/%m/%d").to_string());
    let pre_archive_path = Arc::new(args.archive_path);
    let process_id = Arc::new(args.process_id);
    let error_process_data_path = args.error_process_data_path;

    dotenv().ok();
    let start = Instant::now();
    // if dotenv().is_err() {
    //     eprintln!("Failed to load .env file");
    // }

    // DB Connectivity
    /*
    let conn_str = std::env::var("DB_CONN").expect("DB_CONN not found in env");
    let mgr = bb8_tiberius::ConnectionManager::build(conn_str.as_str())?;
    let pool = bb8::Pool::builder()
        .max_size(
            THREADS
                // (THREADS / 2)
                .try_into()
                .expect("Failed to convert THREADS to u32"),
        )
        .build(mgr)
        .await?;
    let pool = Arc::new(pool);
    */

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

    let company = env::var("CompanyDB").expect("CompanyDB not found in env");
    let username = env::var("UserName").expect("UserName not found in env");
    let password = env::var("Password").expect("Password not found in env");

    /*
    let client = match reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .timeout(Duration::from_secs(120))
        .build()
    {
        Ok(client) => Arc::new(client),
        Err(err) => {
            error!("Failed to build reqwest client: {err}");
            return Err("Failed to build reqwest HTTP client".into());
        }
    };
    */

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

    let token_result = get_token(&company, &username, &password, &client).await;

    if let Ok(token) = &token_result {
        // Handle success
        debug!("Token received: {:?}", token);
        let session_id = Arc::new(["B1SESSION=", &token.session_id].concat());

        // let order_string: &str;
        let mut order_result: Value = match get_warehouse_orders(&session_id, &client).await {
            Ok(orders) => {
                // order_string = &orders.orders;
                orders.orders
            }
            Err(err) => {
                error!("Failed to get Orders {err}");
                return Err(anyhow!("Failed to fetch orders"));
            }
        };

        // info!("Order Result {:?}", order_result);

        let order_vec: Vec<serde_json::Value> = order_result
            .as_array_mut()
            .cloned()
            .expect("Failed to convert order result to array");

        if order_vec.is_empty() {
            // if response.orders.is_empty() {
            warn!("No orders found.");
        } else {
            let semaphore = Arc::new(Semaphore::new(THREADS));
            let mut handles = Vec::new();
            // info!("Response: {response}");
            for mut order in order_vec.into_iter() {
                let semaphore = Arc::clone(&semaphore);
                // let pool_clone = Arc::clone(&pool);
                let sap_pool_clone = Arc::clone(&sap_pool);
                let client_clone = Arc::clone(&client);
                let session_id_clone = Arc::clone(&session_id);
                let pre_archive_path_clone = Arc::clone(&pre_archive_path);
                let dropping_path_clone = Arc::clone(&dropping_path);
                let process_id_clone = Arc::clone(&process_id);
                let mut error_process_data_path_clone = error_process_data_path.clone();
                let handle = task::spawn(async move {
                    // Acquire semaphore with proper error handling
                    let _permit = semaphore
                        .acquire_owned()
                        .await
                        .context("Failed to acquire semaphore")?;

                    match process_data(
                        &mut order,
                        &client_clone,
                        &session_id_clone,
                        &pre_archive_path_clone,
                        &dropping_path_clone,
                        // pool_clone,
                        sap_pool_clone,
                    )
                    .await
                    {
                        Ok(_) => (),
                        Err(err) => {
                            let doc_entry: usize =
                                order.get("DocEntry").unwrap().as_i64().unwrap() as usize;
                            let db_time = chrono::Local::now().format("%Y-%m-%d").to_string();
                            match error_warehouse_order_status(
                                doc_entry,
                                &Arc::clone(&session_id_clone),
                                &db_time,
                                &Arc::clone(&client_clone),
                            )
                            .await
                            {
                                Ok(true) => {
                                    info!(
                                        "Patch request successful with success status for Order {}",
                                        doc_entry
                                    );
                                }
                                Ok(false) => {
                                    error!(
                                        "Patch request failed with non-success status for Order {}!",
                                        doc_entry
                                        );
                                    return Err(anyhow!(
                                            "Patch request failed with non-success status for Order {}!",
                                            doc_entry
                                            ));
                                }
                                Err(e) => {
                                    error!("Error sending patch request: {}", e);
                                    return Err(anyhow!("Error sending patch request: {}", e));
                                }
                            };
                            let error_str = &err.to_string();
                            let reference_number = order.get("DocNum").unwrap().to_string();
                            let timestamp = SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap()
                                .as_millis()
                                .to_string();
                            error!("Failed to process order, {err}");
                            let prism_struct = prism_structure::Output {
                                agency: Agency::ERROR,
                                company: Cow::Borrowed("basicfun"),
                                direction: Some(Cow::Borrowed("OB")),
                                process_id: Cow::Borrowed(&process_id_clone),
                                status: Cow::Borrowed("ERROR"),
                                error_type: Some(Cow::Borrowed("ERP")),
                                error_description: Some(Cow::Borrowed(error_str)),
                                reference: Some(Cow::Borrowed(&reference_number)),
                                plant_name: Some(Cow::Borrowed("BasicFun")),
                                transaction_code: Some(Cow::Borrowed("940")),
                                b2bi_timestamp: Some(Cow::Borrowed(&timestamp)),
                                edi_file_name: None,
                                erp_file_name: None,
                                error_file_name: None,
                                raw_file_name: None,
                            };
                            let prism_string = match quick_xml::se::to_string(&prism_struct) {
                                Ok(prism_string) => prism_string,
                                Err(err) => {
                                    return Err(anyhow!("Failed to serialize Prism Error Process Data for {reference_number}: {err:?}"));
                                }
                            };
                            let file_name = [
                                "processData_",
                                &process_id_clone,
                                "_",
                                &reference_number,
                                "_",
                                &now.format("%Y%m%d_%H%M%S%3f").to_string(),
                                ".xml",
                            ]
                            .concat();
                            error_process_data_path_clone.push(file_name);
                            match std::fs::write(&*error_process_data_path_clone, &prism_string) {
                                Ok(_) => {
                                    info!(
                                        "Error process data {} written successfully",
                                        error_process_data_path_clone.display()
                                    );
                                }
                                Err(err) => {
                                    return Err(anyhow!(
                                        "Failed to write prism data to file {err}"
                                    ));
                                }
                            };
                            return Err(anyhow!("Failed to process order {error_str}"));
                        }
                    };
                    Ok(())
                });
                handles.push(handle);
            }
            let results = join_all(handles).await;
            let error_count = results
                .iter()
                .filter(|result| match result {
                    Ok(Ok(_)) => false,
                    Ok(Err(_)) | Err(_) => true,
                })
                .count();

            // Log the errors
            for result in &results {
                match result {
                    Ok(Err(e)) => eprintln!("Task failed: {e}"),
                    Err(e) => eprintln!("Task panicked: {e}"),
                    _ => {}
                }
            }

            if error_count > 0 {
                eprintln!("{error_count} tasks failed");
                std::process::exit(1);
            }

            /*
            for handle in handles {
                let _ = handle.await.unwrap();
            }
            */
        }
    }
    info!("Processing Time {:?}", start.elapsed());

    Ok(())
}

async fn process_data(
    order: &mut Value,
    client: &ClientWithMiddleware,
    session_id: &str,
    pre_archive_path: &Path,
    dropping_path: &Path,
    // pool: Arc<Pool<bb8_tiberius::ConnectionManager>>,
    sap_pool: Arc<Pool<bb8_tiberius::ConnectionManager>>,
) -> Result<(), anyhow::Error> {
    // let mut conn = pool.get().await?;
    // info!("Order: {}", order);
    /*
    let order_data: Orders = match serde_json::from_value(order.clone()) {
        Ok(data) => data,
        Err(e) => {
            error!("Deserialization error: {}", e);
            // This will show you the path to the field that failed
            return Err(format!("Failed to deserialize: {e}").into());
        }
    };
    */

    info!("Raw Order: {}", order);
    let order_data: Orders =
        match serde_path_to_error::deserialize(order.clone().into_deserializer()) {
            Ok(data) => data,
            Err(err) => return Err(anyhow!("Failed to Deserialize Order: {err}")),
        };

    let current_time = chrono::Local::now();
    let file_time = current_time.format("%Y%m%d_%H%M%S%3f").to_string();
    // let db_time = current_time.format("%m/%d/%Y %H:%M:%S").to_string();
    let db_time = current_time.format("%Y-%m-%d").to_string();
    let _valid_codes = [
        "K00817", "K00055", "K00091", "K00103", "D01961", "D01932", "K00008", "D01865", "D02327",
        "D00036",
    ];

    // if valid_codes

    // SAP reference number
    let reference_number = &order_data.doc_num;
    debug!("Processing Order: {reference_number}");

    // Customer number cardcode
    let _customer_reference = &order_data.card_code;

    // PO Number Validation
    // if order_data.num_at_card.is_none() {
    //     // SAP reference number
    //     // Customer number cardcode
    //     return Err(format!("SO Number = {reference_number}  PO Number missing").into());
    //     // Error Message : PO Number Missing
    // }

    if order_data
        .num_at_card
        .as_deref()
        .unwrap_or("")
        .trim()
        .is_empty()
    {
        return Err(anyhow!("SO Number = {reference_number} PO Number missing"));
    }
    // PO Validation ends

    // PayToCode Billing Address Validation
    // else if order_data.pay_to_code.is_none() {
    else if order_data
        .pay_to_code
        .as_deref()
        .unwrap_or("")
        .trim()
        .is_empty()
    {
        // SAP reference number
        // Customer number cardcode
        // PO number
        // let customer_po = &order_data.num_at_card;
        return Err(anyhow!(
            "SO Number = {reference_number} Billing Address missing"
        ));

        // Error Message : ORDR.PayToCode Missing
    }
    // PayToCode Billing Address Validation Ends

    // ShipToCode Address Validation
    else if order_data
        .ship_to_code
        .as_deref()
        .unwrap_or_default()
        .is_empty()
    {
        // let valid_codes = [
        //     "K00817", "K00055", "K00091", "K00103", "D01961", "D01932", "K00008", "D01865",
        //     "D02327", "D00036", "D00565",
        // ];

        // if valid_codes.contains(&order_data.card_code.as_str()) {
        match &order_data.address_extension {
            Some(addr_ext) if addr_ext.ship_to_street.is_none() => {
                return Err(anyhow!(
                    "SO Number = {reference_number} Shipping Address missing"
                ));
            }
            None => {
                return Err(anyhow!(
                    "SO Number = {reference_number} Shipping Address missing"
                ));
            }
            _ => {}
        }
        // if let Some(addr_ext) = &order_data.address_extension {
        //     if addr_ext.ship_to_street.is_none() {
        //         // SAP reference number
        //         // Customer number cardcode
        //         // PO number
        //         // // Error Message : ORDR.ShipToCode Missing
        //         return Err(format!("SO Number = {reference_number} Shipping Address missing").into());
        //     }
        // }
        // }
        // SAP reference number
        // Customer number cardcode
        // PO number
        // let customer_po = &order_data.num_at_card;
        // else {
        //     return Err(anyhow!(
        //         "SO Number = {reference_number} Shipping Address missing"
        //     ));
        // }

        // Error Message : ORDR.ShipToCode Missing
    }
    // ShipToCode Address Validation Ends

    // Billing type Address Validation
    else if order_data
        .u_billing_type
        .as_deref()
        .unwrap_or("")
        .trim()
        .is_empty()
    {
        // SAP reference number
        // Customer number cardcode
        // PO number
        // let customer_po = &order_data.num_at_card;
        return Err(anyhow!(
            "SO Number = {reference_number} Billing Type missing"
        ));
        // Error Message : Billing Type (U_BillingType) Missing
    }
    // Billing type validation ends

    // Doc Currency Validation
    else if order_data
        .doc_currency
        .as_deref()
        .unwrap_or("")
        .trim()
        .is_empty()
    {
        // SAP reference number
        // Customer number cardcode
        // PO number
        // let customer_po = &order_data.num_at_card;
        return Err(anyhow!(
            "SO Number = {reference_number} Doc Currency missing"
        ));
        // Error Message : Document Currency (ORDR.DocCur) Missing
    }
    // Billing type validation ends

    // Shipping Type Validation
    else if let Some(code) = order_data.u_transportation_code {
        if code == -1 {
            return Err(anyhow!(
                "SO Number = {reference_number} Shipping Type missing"
            ));
            // Do something when it's -1
        } else if code == 9 {
            return Err(anyhow!("SO Number = {reference_number} Invalid Carrier"));
        }
    } else {
        // VAT Code Validation starts
        // let vat_code_query = "SELECT LicTradNum FROM OCRD WHERE CardCode = @P1";
        // let result = send_query(&sap_pool, vat_code_query, &[&order_data.card_code]).await?;
        // debug!("Select OCRD Query Result: {:?}", result);
        // if result.is_empty() {
        //     return Err(format!("SO Number = {reference_number} VAT Code missing").into());
        // }
        // match result.first() {
        //     Some(row) => {}
        //     None => {
        //         // SAP reference number
        //         // Customer number cardcode
        //         // PO number
        //         // let customer_po = &order_data.num_at_card;
        //         return Err(format!("SO Number = {reference_number} VAT Code missing").into());
        //         // Error Message : VAT Code (OCRD.LicTradNum) Missing
        //     }
        // }
        // // ends VAT Code validation

        let mut shipping_windows = false;
        let amazon_walmart_cardcodes = [
            "K00008", "K00009", "K00010", "D00030", "D01864", "D01865", "D02049", "D02050",
            "D02054", "D02055", "K00007", "K01916", "K01937", "EU00002", "EU00003", "EU00004",
            "EU00005", "EU00006", "EU00007", "EU00008", "EU00009", "K00103", "K00434", "D00808",
            "D01961", "D02327",
        ];
        // let mut amazon_walmart_item_check = false;
        // if amazon_walmart_cardcodes.contains(&order_data.card_code.as_str()){
        //     if order_data.u_tbd_cust_no.is_none(){
        //         amazon_walmart_item_check = true;
        //     }
        // }
        let mut vat_code: bool = false;
        for i in &order_data.document_lines {
            // if i.bar_code.is_none() {
            // info!("i.bar_code value does not exist, fetching");
            // let hts_query = "SELECT CodeBars,U_EAN, U_TBD_GT14 FROM OITM WHERE ItemCode = @P1";
            // let result = send_query(&sap_pool, hts_query, &[&i.item_code]).await?;
            // info!("Select HTS Code Query Result: {:?}", result);
            // if let Some(row) = result.first() {
            //     if let Ok(Some(v)) = row.try_get::<&str, _>("U_EAN") {
            //         info!("EAN Value: {v}");
            //         insert_order_item_value(order, &i.item_code, &i.line_num, "BarCode", v)
            //             .await;
            //     }
            // else if let Ok(Some(v)) = row.try_get::<&str, _>("CodeBars") {
            //     insert_order_item_value(order, &i.item_code, &i.line_num, "BarCode", v)
            //         .await;
            // }
            // else if let Ok(Some(v)) = row.try_get::<&str, _>("U_TBD_GT14") {
            //     insert_order_item_value(order, &i.item_code, &i.line_num, "BarCode", v)
            //         .await;
            // }
            // }
            // }
            // else {
            // info!(
            // "i.bar_code value already exists: {}",
            // i.bar_code.as_ref().unwrap()
            // );
            // }
            if i.u_acw_deliverfrom.is_none() || i.u_acw_deliveryend.is_none() {
                shipping_windows = true;
            } else if i.quantity == 0.0 {
                // Quantity Validation fails
                // SAP reference number
                // Customer number cardcode
                // PO number
                return Err(anyhow!(
                    "SO Number = {reference_number}, Quantity missing for item {}",
                    i.item_code
                ));

                // Error Message : Quantity (RDR1.Quantity) equals to 0
            } else if i.warehouse_code.trim().is_empty() {
                // Warehouse  Validation fails
                // SAP reference number
                // Customer number cardcode
                // PO number
                return Err(anyhow!(
                    "SO Number = {reference_number}, Warehouse code missing for item {}",
                    i.warehouse_code
                ));
                // Error Message : Warehouse Code (RDR1.Whscode) Missing
            } else if i.unit_price < 0.0 {
                // Unit Price Validation fails
                // SAP reference number
                // Customer number cardcode
                // PO number
                return Err(anyhow!(
                    "SO Number = {reference_number}, Unit pricess less than 0  for item {}",
                    i.unit_price
                ));

                // Error Message : Unit Price (RDR1.Price) less than 0
            } else if i.warehouse_code == "PIL" || i.warehouse_code == "PILEU" {
                let hts_query = "SELECT U_HTSUK,U_HTSEU FROM OITM WHERE ItemCode = @P1";
                let result = send_query(&sap_pool, hts_query, &[&i.item_code]).await?;
                debug!("Select HTS Code Query Result: {:?}", result);
                match result.first() {
                    Some(_row) => {}
                    None => {
                        // SAP reference number
                        // Customer number cardcode
                        // PO number
                        // Error Message : HTS Codes PIL / PILEU (OITM.U_HTSEU/OITM.U_HTSUK) Missing
                        return Err(anyhow!(
                            "SO Number = {reference_number}, PIL U_HTSUK,U_HTSEU missing for item {}",
                            i.unit_price
                        ));
                    }
                }
            } else if i.tax_code.is_none() || i.tax_code.as_ref().unwrap() != "Exempt" {
                vat_code = true;
            }

            if amazon_walmart_cardcodes.contains(&order_data.card_code.as_str())
                && i.supplier_cat_num.is_none()
                && i.u_tbd_cust_no.is_none()
            {
                let hts_query =
                    "SELECT Substitute from OSCN where ItemCode = @P1 and CardCode = @P2";
                let result =
                    send_query(&sap_pool, hts_query, &[&i.item_code, &order_data.card_code])
                        .await?;
                debug!("Select Amazon Walmart Catalog Query Result: {:?}", result);
                if result.is_empty() {
                    return Err(anyhow!(
                    "SO Number = {reference_number}, AMAZON AND WALMART SPECIFIC HTS Code missing for item {}",
                    i.item_code
                ));
                }
                // match result.first() {
                //     Some(row) =>  {}
                //     None => {
                //         // SAP reference number
                //         // Customer number cardcode
                //         // PO number
                //         // Error Message : AMAZON WALMART Catalog Log Missing
                //         return Err(format!(
                //             "SO Number = {reference_number}, AMAZON AND WALMART SPECIFIC HTS Code missing for item {}",
                //             i.item_code)
                //         .into());
                //     }
                // }
            }
        }

        if vat_code {
            let vat_code_query = "SELECT LicTradNum FROM OCRD WHERE CardCode = @P1";
            let result = send_query(&sap_pool, vat_code_query, &[&order_data.card_code]).await?;
            debug!("Select OCRD Query Result: {:?}", result);
            if result.is_empty() {
                return Err(anyhow!("SO Number = {reference_number} VAT Code missing"));
            }
        }

        if shipping_windows
            && (order_data.u_begin_window_date.is_none() || order_data.u_end_window_date.is_none())
        {
            // Shipping Window Dates Validation fails
            // SAP reference number
            // Customer number cardcode
            // PO number
            return Err(anyhow!(
                "SO Number = {reference_number}, Shipping Window dates missing for item !",
            ));

            // Error Message : Shipping Window Dates (RDR1.Price) Missing
        }
    }

    // Pre-Archive JSON
    tokio::fs::create_dir_all(&pre_archive_path).await?;
    match save_warehouse_order_json(pre_archive_path, order, &order_data, &file_time, false).await {
        Ok(_) => debug!("Pre-archive completed successfully"),
        Err(err) => return Err(anyhow!("Failed to save JSON: {err}")),
    }

    match update_warehouse_order_status(order_data.doc_entry, session_id, &db_time, client).await {
        Ok(true) => {
            info!(
                "Patch request successful with success status for Order {}",
                order_data.doc_entry
            );
            /*
            let valid_codes = [
                "K00817", "K00055", "K00091", "K00103", "D01961", "D01932", "K00008", "D01865",
            ];
            */
            debug!("Running SELECT Query");
            let warehouse = match order_data.document_lines.first() {
                Some(document_line) => &document_line.warehouse_code,
                None => {
                    error!("No warehouse exists for order {}", order_data.doc_entry);
                    return Err(anyhow!(
                        "Failed to fetch billing info for order {}",
                        order_data.doc_entry
                    ));
                }
            };
            if warehouse == "PIL" || warehouse == "PILEU" {
                for i in &order_data.document_lines {
                    // if i.bar_code.is_none() {
                    info!("i.bar_code value does not exist, fetching");
                    let hts_query =
                        "SELECT CodeBars,U_EAN, U_TBD_GT14 FROM OITM WHERE ItemCode = @P1";
                    let result = send_query(&sap_pool, hts_query, &[&i.item_code]).await?;
                    info!("Select HTS Code Query Result: {:?}", result);
                    if let Some(row) = result.first() {
                        if let Ok(Some(v)) = row.try_get::<&str, _>("U_EAN") {
                            info!("EAN Value: {v}");
                            insert_order_item_value(order, &i.item_code, &i.line_num, "BarCode", v)
                                .await;
                        }
                    }
                }
            }
            let _ =
                fetch_hts_codes_data(Arc::clone(&sap_pool), &order_data, order, warehouse).await;

            let _ = fetch_billing_data(
                sap_pool,
                &order_data,
                order,
                warehouse,
                &order_data.card_code,
                order_data.doc_entry,
            )
            .await;

            info!("Saving Order {}", order);
            match save_warehouse_order_json(dropping_path, order, &order_data, &file_time, true)
                .await
            {
                Ok(_) => {
                    debug!("Post-Patch File saving successful");
                }
                Err(err) => return Err(anyhow!("Failed to save JSON: {err}")),
            }
        }
        Ok(false) => {
            error!(
                "Patch request failed with non-success status for Order {}!",
                order_data.doc_entry
            );
            return Err(anyhow!(
                "Patch request failed with non-success status for Order {}!",
                order_data.doc_entry
            ));
        }
        Err(e) => {
            error!("Error sending patch request: {}", e);
            return Err(anyhow!("Error sending patch request: {}", e));
        }
    }
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

    // info!("Login Data: {}", login_data);

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

    debug!("Token: {:?}", response);

    // Deserialize the response JSON into the Token struct
    let token: Token = match response.json().await {
        Ok(token) => token,
        Err(err) => {
            return Err(format!("Failed to deserialize JSON for token, {err}").into());
        }
    }; // Parse the response body

    Ok(token) // Return the deserialized token
}

async fn get_warehouse_orders(
    session_id: &str,
    client: &ClientWithMiddleware,
) -> Result<Root, reqwest_middleware::Error> {
    // Fetching Order Data for 940
    // let url = "https://BFAZWSAP01.corp.basicfun.com:50000/b1s/v1/Orders?$filter=U_Warehouse_Order eq 'Y'";
    let uri = "/b1s/v1/Orders";
    let query = "$filter=U_Warehouse_Order eq 'Y' AND DocumentStatus eq 'bost_Open'";
    let url = format!("{BASE_URL}{uri}?{query}");
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(
        COOKIE,
        HeaderValue::from_str(session_id).expect("Failed to create COOKIE header"),
    );

    // let response = match client.get(url).headers(headers).send().await {
    let response = match send_request(client, Method::GET, url, Bytes::new(), headers).await {
        Ok(response) => {
            if response.status().is_success() {
                match response.json::<Root>().await {
                    // match response.text().await {
                    Ok(data) => {
                        debug!("Data: {}", data.orders.to_string());
                        data
                    }
                    Err(e) => {
                        eprintln!("Failed to deserialize JSON: {e}");
                        return Err(reqwest_middleware::Error::Reqwest(e));
                    }
                }
            } else {
                // Handle non-success status codes (e.g., 404, 500, etc.)
                error!(
                    "Fetch 940 request failed with status: {}",
                    response.status()
                );
                return Err(reqwest_middleware::Error::Reqwest(
                    response.error_for_status().unwrap_err(),
                ));
            }
        }
        Err(err) => {
            error!("Failed to get token, {err}");
            return Err(err);
        }
    };
    // info!("Response: {:?}", response);
    // let order_json: Root = serde_json::from_str(&response).expect("Failed to deserialize response");

    Ok(response)
}

async fn save_warehouse_order_json(
    path: &Path,
    order_data: &Value,
    order: &Orders,
    current_time: &str,
    as_xml: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // async fn save_warehouse_order_json(path: &Path, order: &Orders) -> Result<(), Box<dyn Error>> {
    let warehouse = match order.document_lines.first() {
        Some(document_line) => &document_line.warehouse_code,
        None => {
            error!("No warehouse exists for order {}", order.doc_entry);
            return Err(format!(
                "Failed to fetch warehouse info for order {}",
                order.doc_entry
            )
            .into());
        }
    };
    let extension = if as_xml { "xml" } else { "json" };
    let json_location = path.join(format!(
        "{}-940_order_{}-{}.{}",
        warehouse,
        // order.card_code.replace(" ", "_").replace(",", ""),
        // order.bplname,
        order.doc_num,
        current_time,
        extension
    ));

    debug!(
        "JSON File Path: {}",
        json_location.as_os_str().to_str().unwrap()
    );
    if as_xml {
        debug!("Order Data to convert to XML: {order_data}");
        let order_data = normalize_numbers(order_data.clone());
        let mut order_xml =
            to_string_with_root("root", &order_data).expect("Failed to serialize to XML");

        order_xml = order_xml
            .replace("\r\n ", " ")
            .replace("\r\n", " ")
            .replace("\r ", " ")
            .replace("\n ", " ")
            .replace(['\r', '\n'], " ");
        order_xml.push('\n');

        write(json_location, order_xml)
            .await
            .expect("Unable to write XML file");
    } else {
        write(json_location, &order_data.to_string())
            .await
            .expect("Unable to write JSON file");
    }

    // let file = File::create(json_location).expect("Unable to create JSON file.");
    // serde_json::to_writer(file, &order_data).expect("Unable to write JSON file");
    Ok(())
}

/*
async fn update_shipping_information(
    pool: Arc<Pool<bb8_tiberius::ConnectionManager>>,
    order_data: &Orders,
    order: &mut Value,
) -> Result<(), Box<dyn std::error::Error>> {
    let query = "SELECT TOP 1
    Ship_To_Name, Ship_To_Address, Ship_To_City,
    Ship_To_State, Ship_To_Country, Ship_To_Zipcode
    FROM DropShipOrders
    WHERE PO_Number = @P1
    ORDER BY DSO_Id DESC";

    // let mut stream = conn.query(scac_query, &[u_ship_scac]).await?;
    // let mut conn = pool.get().await?;
    let result = send_query(
        &pool,
        query,
        &[&order_data.num_at_card],
        // [SqlType::String(&order_data.num_at_card)].to_vec(),
    )
    .await?;
    debug!("First Select Query Result: {:?}", result);
    match result.first() {
        Some(row) => {
            alter_order_value(
                order,
                "ShipToAddress2",
                row.get::<&str, _>("Ship_To_Name").unwrap_or_default(),
                "AddressExtension",
            )
            .await;

            alter_order_value(
                order,
                "ShipToStreet",
                row.get::<&str, _>("Ship_To_Address").unwrap_or_default(),
                "AddressExtension",
            )
            .await;
            alter_order_value(
                order,
                "ShipToCity",
                row.get::<&str, _>("Ship_To_City").unwrap_or_default(),
                "AddressExtension",
            )
            .await;
            alter_order_value(
                order,
                "ShipToState",
                row.get::<&str, _>("Ship_To_State").unwrap_or_default(),
                "AddressExtension",
            )
            .await;
            alter_order_value(
                order,
                "ShipToCountry",
                row.get::<&str, _>("Ship_To_Country").unwrap_or_default(),
                "AddressExtension",
            )
            .await;
            alter_order_value(
                order,
                "ShipToZipCode",
                row.get::<&str, _>("Ship_To_Zipcode").unwrap_or_default(),
                "AddressExtension",
            )
            .await;
        }
        None => {
            error!("Failed to fetch first result from Shipping Address Query");
            return Err(anyhow::Error::msg(
                "Failed to fetch first result from Shipping Address Query",
            )
            .into());
        }
    }

    Ok(())
}
*/

/*
async fn update_billto_information(
    sap_pool: Arc<Pool<bb8_tiberius::ConnectionManager>>,
    order_data: &Orders,
    order: &mut Value,
) -> Result<(), Box<dyn std::error::Error>> {
    let query = "SELECT TOP 1
                            Street,Block,Address2,City,
                            ZipCode,State,Country from CRD1
                            where CardCode = @P1
                            and AdresType = 'B'";

    let result = send_query(&sap_pool, query, &[&order_data.card_code]).await?;
    // (query, &[&order_data.card_code]).await?;
    match result.first() {
        Some(row) => {
            alter_order_value(
                order,
                "BillToStreet",
                row.get::<&str, _>("Street").unwrap_or_default(),
                "AddressExtension",
            )
            .await;

            alter_order_value(
                order,
                "BillToBlock",
                row.get::<&str, _>("Block").unwrap_or_default(),
                "AddressExtension",
            )
            .await;
            alter_order_value(
                order,
                "BillToStreetNo",
                row.get::<&str, _>("Address2").unwrap_or_default(),
                "AddressExtension",
            )
            .await;
            alter_order_value(
                order,
                "BillToCity",
                row.get::<&str, _>("City").unwrap_or_default(),
                "AddressExtension",
            )
            .await;
            alter_order_value(
                order,
                "BillToZipCode",
                row.get::<&str, _>("ZipCode").unwrap_or_default(),
                "AddressExtension",
            )
            .await;
            alter_order_value(
                order,
                "BillToState",
                row.get::<&str, _>("State").unwrap_or_default(),
                "AddressExtension",
            )
            .await;
            alter_order_value(
                order,
                "BillToCountry",
                row.get::<&str, _>("Country").unwrap_or_default(),
                "AddressExtension",
            )
        }
        None => {
            error!("Failed to fetch first result from Billing Address Query");
            return Err(anyhow::Error::msg(
                "Failed to fetch first result from Billing Address Query",
            )
            .into());
        }
    }
    .await;
    Ok(())
}
*/

async fn fetch_hts_codes_data(
    sap_pool: Arc<Pool<bb8_tiberius::ConnectionManager>>,
    order_data: &Orders,
    order: &mut Value,
    warehouse: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    /*
       Extracting the warehouse country from db using warehouse code
    */
    let whse_country_query = "select Country 
    from OWHS
    where WhsCode = @P1";

    let result = send_query(&sap_pool, whse_country_query, &[&warehouse.to_string()]).await?;

    let mut warehouse_country: String = String::new();

    for row in result {
        debug!(
            "Assigning Warehouse country Column Value from Row: {:?}",
            row
        );
        warehouse_country = row
            .get::<&str, _>("Country")
            .expect("Failed to Fetch Country from Query Response")
            .to_string();
    }

    /*
       Using above country and each item code to find HTS code against each item
    */
    for line in &order_data.document_lines {
        let hts_code_query = "select U_HTSCode 
        from [@ECSB1_HTSCODES]
        where U_ItemCode = @P1
        AND U_Country = @P2";

        let result = send_query(
            &sap_pool,
            hts_code_query,
            &[&line.item_code.to_string(), &warehouse_country.to_string()],
        )
        .await?;

        let mut hts_code: String = String::new();

        for row in result {
            debug!(
                "Assigning Warehouse country Column Value from Row: {:?}",
                row
            );
            hts_code = row
                .get::<&str, _>("U_HTSCode")
                .expect("Failed to Fetch HTS Code from Query Response")
                .to_string();
        }
        insert_order_item_value(
            order,
            &line.item_code.to_string(),
            &line.line_num,
            "U_HTS_Code",
            &hts_code,
        )
        .await;
        /*
           Extracting Carton Quantity against each item
        */
        let carton_qty_query = "select PurPackUn
                                from OITM
                                where ItemCode = @P1";
        let carton_result =
            send_query(&sap_pool, carton_qty_query, &[&line.item_code.to_string()]).await?;
        let mut carton_qty: String = String::new();
        for row in carton_result {
            debug!(
                "Assigning Warehouse country Column Value from Row: {:?}",
                row
            );
            if let Some(decimal) = row.get::<Decimal, _>("PurPackUn") {
                // Use rust_decimal::Decimal directly and convert to string
                carton_qty = decimal.to_string(); // rust_decimal provides to_string()
            } else {
                debug!("PurPackUn was NULL for ItemCode: {}", line.item_code);
                carton_qty = "0".to_string(); // Default value if NULL
            }
        }
        insert_order_item_value(
            order,
            &line.item_code.to_string(),
            &line.line_num,
            "U_Carton_Qty",
            &carton_qty,
        )
        .await;
    }

    // match order_data.document_lines {
    //     Some(ref document_lines) =>{
    //         for line in document_lines {
    //             if let Some(ref item_code) = line.ItemCode {
    //                 println!("ItemCode: {}", item_code);
    //             } else {
    //                 println!("ItemCode not found for this line.");
    //             }
    //         }
    //     }
    // }
    Ok(())
}

async fn fetch_billing_data(
    pool: Arc<Pool<bb8_tiberius::ConnectionManager>>,
    order_data: &Orders,
    order: &mut Value,
    warehouse: &str,
    card_code: &str,
    number: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    // TODO: Billing Data
    /*
        TODO: SCENARIO 1
        // u_ship_scac.is_some()
        select U_SCAC
        from OSHP
        where TrnspCode = {U_SHIP_SCAC}

        POPULATE u_ship_scac

        // After Fetching or in parallel with U_SCAC
        select U_Account_No
        from [@ECSB1_SHIPVIA]
        where U_CardCode = {card_code}
        and U_SCAC = {U_SCAC}
        and U_BPM_Ship_VIA = {U_SHIP_INTERSTAT}
        and U_Warehouse = {warehouse}

        POPULATE u_ship_via_acct

        TODO: SCENARIO 2
        // u_ship_scac.is_none() && u_ship_interstat.is_some()
        select U_SCAC, U_Account_No
        from [@ECSB1_SHIPVIA]
        where U_CardCode = {card_code}
        and U_BPM_Ship_VIA = {U_SHIP_INTERSTAT}
        and U_Warehouse = {warehouse}

        POPULATE u_ship_via_acct

        // Fetch Shipment Method (use U_SCAC from 1st query)
        select U_SCAC
        from OSHP
        where TrnspCode = {U_SCAC}

        POPULATE u_ship_scac
    */

    /*
       UPDATED SCENARIOS
       SCENARIO 1 :
       WHEN u_ship_scac is some:

       select U_SCAC
       from OSHP
       where TrnspCode = u_ship_scac
       select TOP 1 U_Account_No
       from [@ECSB1_SHIPVIA]
       and U_SCAC = u_ship_scac
       and U_CardCode = card_code
       and U_Warehouse = warehouse

       -If ORDR.U_BillingType = ""Prepaid"" or ""Prepaid Bill"" then
        SELECT account_no
        from [dbo].[@TBD_CARRIER_ACT_NUM] where
        whsecode = warehouse and
        U_method = OSHP.U_Method

    */
    info!("Fetching Billing Data for Order {number}");
    let valid_codes = [
        "K00817", "K00055", "K00091", "K00103", "D01961", "D01932", "K00008", "D01865", "D02327",
        "D00036", "D00565",
    ];

    if order_data
        .address_extension
        .as_ref()
        .and_then(|ext| ext.bill_to_street.as_deref())
        .is_none_or(|s| s.is_empty())
    {
        // }
        // if valid_codes.contains(&order_data.card_code.as_str()) {
        debug!("Checking Dropship Customer Bill to Name");
        let address_query = "SELECT CRD1.Street,CRD1.Address,CRD1.Block,CRD1.Address2,CRD1.City,CRD1.State,CRD1.ZipCode,CRD1.Country
            FROM ORDR
            INNER JOIN CRD1 ON ORDR.CardCode = CRD1.CardCode
            WHERE CRD1.Address = ORDR.PayToCode
            AND ORDR.DocNum = @P1
            AND CRD1.AdresType = 'B'";
        let address_result =
            send_query(&pool, address_query, &[&order_data.doc_num.to_string()]).await?;
        debug!("Billing Address Result: {:?}", address_result);
        for row in address_result {
            if let Some(u_bill_to_address) = row.get::<&str, _>("Street") {
                // insert_order_value(order, "U_Bill_To_Name", u_bill_to_address).await;
                add_to_address_extension(order, "BillToStreet", u_bill_to_address).await;
            }
            if let Some(u_bill_to_block) = row.get::<&str, _>("Block") {
                // insert_order_value(order, "U_Bill_To_Name", u_bill_to_address).await;
                add_to_address_extension(order, "BillToBlock", u_bill_to_block).await;
            }
            if let Some(u_bill_to_city) = row.get::<&str, _>("City") {
                // insert_order_value(order, "U_Bill_To_Name", u_bill_to_address).await;
                add_to_address_extension(order, "BillToCity", u_bill_to_city).await;
            }
            if let Some(u_bill_to_state) = row.get::<&str, _>("State") {
                // insert_order_value(order, "U_Bill_To_Name", u_bill_to_address).await;
                add_to_address_extension(order, "BillToState", u_bill_to_state).await;
            }
            if let Some(u_bill_to_zip_code) = row.get::<&str, _>("ZipCode") {
                // insert_order_value(order, "U_Bill_To_Name", u_bill_to_address).await;
                add_to_address_extension(order, "BillToZipCode", u_bill_to_zip_code).await;
            }
            if let Some(u_bill_to_country) = row.get::<&str, _>("Country") {
                // insert_order_value(order, "U_Bill_To_Name", u_bill_to_address).await;
                add_to_address_extension(order, "BillToCountry", u_bill_to_country).await;
            }
            if let Some(u_bill_to_address2) = row.get::<&str, _>("Address2") {
                // insert_order_value(order, "U_Bill_To_Name", u_bill_to_address).await;
                add_to_address_extension(order, "BillToAddress2", u_bill_to_address2).await;
            }
        }
    }
    match order_data.u_transportation_code {
        Some(ref transportation_code) => {
            debug!("Checking Dropship Customer Bill to Name");
            let address_query = "SELECT CRD1.Street
                FROM ORDR
                INNER JOIN CRD1 ON ORDR.CardCode = CRD1.CardCode
                WHERE CRD1.Address = ORDR.PayToCode
                AND ORDR.DocNum = @P1
                AND CRD1.AdresType = 'B'";
            debug!(
                "SELECT CRD1.Street FROM ORDR INNER JOIN CRD1 ON ORDR.CardCode = CRD1.CardCode WHERE CRD1.Address = ORDR.PayToCode AND ORDR.DocNum = {} AND CRD1.AdresType = 'B'",
                order_data.doc_num
            );
            let address_result =
                send_query(&pool, address_query, &[&order_data.doc_num.to_string()]).await?;
            debug!("Address Result: {:?}", address_result);
            for row in address_result {
                if let Some(u_bill_to_address) = row.get::<&str, _>("Street") {
                    insert_order_value(order, "U_Bill_To_Name", u_bill_to_address).await;
                }
            }
            if valid_codes.contains(&order_data.card_code.as_str()) {
                debug!("Dropship Customers");
                let scac_query = "select TrnspName, U_SCAC, U_QualCode
                        from OSHP
                        where TrnspCode = @P1";
                let result = send_query(&pool, scac_query, &[transportation_code]).await?;
                debug!("First Select Query Result: {:?}", result);
                for row in result {
                    debug!("Assigning SCENARIO 1 U_SCAC Value for Row: {:?}", row);
                    // ERROR IF GET FAILS
                    if let Some(u_scac) = row.get::<&str, _>("U_SCAC") {
                        insert_order_value(order, "U_SCAC", u_scac).await;
                    }
                    if let Some(ship_code) = row.get::<&str, _>("U_QualCode") {
                        insert_order_value(order, "U_ShipCode_W6602", ship_code).await;
                    }
                    if let Some(transp_name) = row.get::<&str, _>("TrnspName") {
                        insert_order_value(order, "U_TrnspName_W6605", transp_name).await;
                    }
                }
                let account_query = "
                                    select top 1 u_account_no, u_account_zipcode
                                    from [@ecsb1_shipvia]
                                    where u_scac = @P1
                                    and u_cardcode = @P2
                                    and u_warehouse = @P3";
                let result = send_query(
                    &pool,
                    account_query,
                    &[transportation_code, &card_code, &warehouse],
                )
                .await?;
                // conn.query(scac_query, &[u_ship_scac]).await?;
                debug!("2nd Select Query Result: {:?}", result);
                for row in result {
                    debug!("Assigning SCENARIO 1 U_Account_No Value for Row: {:?}", row);
                    if let Some(u_account_no) = row.get::<&str, _>("u_account_no") {
                        insert_order_value(order, "U_Account_No", u_account_no).await;
                    }
                    if let Some(u_account_zip) = row.get::<&str, _>("u_account_zipcode") {
                        insert_order_value(order, "U_Account_Zipcode", u_account_zip).await;
                    }
                }
            } else {
                match order_data.u_ship_scac {
                    Some(ref u_ship_scac) => {
                        debug!("U_SHIP_SCAC is Some");
                        let scac_query = "select TrnspName, U_SCAC, U_QualCode
                                from OSHP
                                where TrnspCode = @P1";

                        let result = send_query(&pool, scac_query, &[u_ship_scac]).await?;
                        debug!("First Select Query Result: {:?}", result);

                        for row in result {
                            debug!("Assigning SCENARIO 1 U_SCAC Value for Row: {:?}", row);
                            if let Some(u_scac) = row.get::<&str, _>("U_SCAC") {
                                insert_order_value(order, "U_SCAC", u_scac).await;
                            }

                            if let Some(ship_code) = row.get::<&str, _>("U_QualCode") {
                                insert_order_value(order, "U_ShipCode_W6602", ship_code).await;
                            }
                            if let Some(transp_name) = row.get::<&str, _>("TrnspName") {
                                insert_order_value(order, "U_TrnspName_W6605", transp_name).await;
                            }
                        }

                        // let mut conn_2 = pool.get().await?;
                        debug!(
                            "
                            select top 1 u_account_no, u_account_zipcode
                            from [@ecsb1_shipvia]
                            where u_scac = {u_ship_scac}
                            and u_cardcode = {card_code} 
                            and u_warehouse = {warehouse}",
                        );

                        let account_query = "
                                    select top 1 u_account_no, u_account_zipcode
                                    from [@ecsb1_shipvia]
                                    where u_scac = @P1
                                    and u_cardcode = @P2
                                    and u_warehouse = @P3";
                        let result = send_query(
                            // &mut conn_2,
                            &pool,
                            account_query,
                            &[u_ship_scac, &card_code, &warehouse],
                        )
                        .await?;
                        // conn.query(scac_query, &[u_ship_scac]).await?;
                        debug!("2nd Select Query Result: {:?}", result);
                        for row in result {
                            debug!("Assigning SCENARIO 1 U_Account_No Value for Row: {:?}", row);
                            if let Some(u_account_no) = row.get::<&str, _>("U_Account_No") {
                                insert_order_value(order, "U_Account_No", u_account_no).await;
                            }
                            if let Some(u_account_zip) = row.get::<&str, _>("U_Account_Zipcode") {
                                insert_order_value(order, "U_Account_Zipcode", u_account_zip).await;
                            }
                        }
                    }
                    // Some(ref u_ship_scac) => {
                    //     info!(
                    //         "Card Code {} in valid_codes for {u_ship_scac}",
                    //         &order_data.card_code
                    //     )
                    // }
                    None => match (
                        &order_data.u_transportation_code,
                        &order_data.u_billing_type,
                    ) {
                        (Some(u_transportation_code), Some(u_billing_type)) => {
                            debug!("u_transportation_code: {u_transportation_code} & u_billing_type: {u_billing_type}");
                            // let mut conn = pool.get().await?;
                            let scac_query = "select TrnspName, U_SCAC, U_Method, U_QualCode 
                            from OSHP
                            where TrnspCode = @P1";
                            let result = send_query(
                                &pool,
                                scac_query,
                                &[&u_transportation_code.to_string()],
                            )
                            .await?;
                            let mut new_u_method: String = String::new();
                            for row in result {
                                debug!(
                                    "Assigning SCENARIO 2 U_SCAC, U_Method Column Values from Row: {:?}",
                                    row
                                );
                                /*
                                new_u_method = row
                                    .get::<&str, _>("U_Method")
                                    .expect("Failed to Fetch U_Method from Query Response")
                                    .to_string();
                                */

                                if let Some(temp_u_method) = row.get::<&str, _>("U_Method") {
                                    new_u_method = temp_u_method.to_owned();
                                    insert_order_value(order, "U_Method", &new_u_method).await;
                                }

                                if let Some(u_scac) = row.get::<&str, _>("U_SCAC") {
                                    insert_order_value(order, "U_SCAC", u_scac).await;
                                }

                                if let Some(ship_code) = row.get::<&str, _>("U_QualCode") {
                                    insert_order_value(order, "U_ShipCode_W6602", ship_code).await;
                                }
                                if let Some(transp_name) = row.get::<&str, _>("TrnspName") {
                                    insert_order_value(order, "U_TrnspName_W6605", transp_name)
                                        .await;
                                }
                            }

                            /* Below query is for extracting account no */
                            let billing_type = u_billing_type.to_string();
                            if billing_type == "PREPAID"
                                || billing_type == "Prepaid Bill"
                                || billing_type == "3RD PARTY"
                            {
                                let account_query = "
                                select U_CarrierAcctNum
                                from [@TBD_CARRIER_ACT_NUM]
                                where U_Carrier = @P1
                                and U_WhsCode = @P2";
                                if !new_u_method.is_empty() {
                                    let result = send_query(
                                        &pool,
                                        account_query,
                                        &[&new_u_method, &warehouse],
                                    )
                                    .await?;
                                    // conn.query(scac_query, &[u_ship_scac]).await?;
                                    for row in result {
                                        debug!(
                                            "Assigning SCENARIO 2 U_Account_No Value from Row: {:?}",
                                            row
                                        );
                                        insert_order_value(
                                            order,
                                            "U_Account_No",
                                            row.get::<&str, _>("U_CarrierAcctNum").expect(
                                                "Failed to fetch U_Account_No from Query Result",
                                            ),
                                        )
                                        .await;
                                    }
                                }
                            }
                        }
                        _ => {
                            error!("U_SHIP_SCAC, u_transportation_code, u_billing_type are NULL, cannot process order",);
                            return Err(anyhow::Error::msg(
                                "U_SHIP_SCAC, u_transportation_code, u_billing_type are NULL, cannot process order",
                            )
                            .into());
                        }
                    },
                };
            }
        }
        None => {
            info!(
                "Transportation Code is not populated in order {}",
                order_data.doc_entry
            );
        }
    };

    let pay_to_code_query = "SELECT
                        o.PayToCode COLLATE Latin1_General_CI_AS AS PayToCode,
                        c.Address COLLATE Latin1_General_CI_AS AS Address,
                        c.AdresType COLLATE Latin1_General_CI_AS AS AdresType,
                        c.Street COLLATE Latin1_General_CI_AS AS Street
                        FROM
                            ORDR O
                        JOIN
                            CRD1 C ON O.CardCode = C.CardCode
                        WHERE
                            C.AdresType = 'B'
                            AND O.PayToCode = C.Address
                            and O.DocEntry = @P1";
    let pay_to_code_result = send_query(
        &pool,
        pay_to_code_query,
        &[&order_data.doc_entry.to_string()],
        // [SqlType::String(&order_data.num_at_card)].to_vec(),
    )
    .await?;

    debug!("First Select Query Result: {:?}", pay_to_code_result);
    for row in pay_to_code_result {
        debug!("Assigning SCENARIO 1 U_SCAC Value for Row: {:?}", row);
        if let Some(address) = row.get::<&str, _>("Street") {
            insert_order_value(order, "U_Address", address).await;
        }
    }

    let dropship_query = "SELECT CONVERT(NVARCHAR(50), NaturalPer) AS NaturalPer FROM OCRD WHERE CardCode = @P1 COLLATE Latin1_General_100_CI_AS;";
    let dropship_result = send_query(&pool, dropship_query, &[&card_code]).await?;
    debug!("Dropship Query Result: {:?}", dropship_result);

    for row in dropship_result {
        if let Some(natural_per) = row.get::<&str, _>("NaturalPer") {
            debug!(
                "NaturalPer value for CardCode {}: {}",
                order_data.card_code, natural_per
            );
            insert_order_value(order, "U_Dropship_Person", natural_per).await;
        }
    }
    // TODO: Update Billing info in JSON
    Ok(())
}

async fn error_warehouse_order_status(
    number: usize,
    session_id: &str,
    current_time: &str,
    client: &ClientWithMiddleware,
) -> Result<bool, reqwest_middleware::Error> {
    let uri = format!("/b1s/v1/Orders({number})");

    let body = json!({
        // "U_Warehouse_Order": "Y",
        "U_Warehouse_Order": "E",
        "U_Warehouse_Order_Date": current_time,
        "U_Warehouse_Order_Process": "940 Generation",
    })
    .to_string();

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(
        COOKIE,
        HeaderValue::from_str(session_id).expect("Failed to create COOKIE header"),
    );

    let url = format!("{BASE_URL}{uri}");
    debug!("Patch Body to {url} for Order {number}: {body}");

    match send_request(client, Method::PATCH, url, Bytes::from(body), headers).await {
        Ok(response) => {
            let status = response.status();
            let response_body = response
                .text()
                .await
                .unwrap_or_else(|_| "Failed to read response body".to_string());

            if status.is_success() {
                debug!(
                    "Error patch request successfully sent for order {}, status code {}",
                    number, status
                );
                Ok(true)
            } else {
                error!(
                    "Error patch request failed for order {}, status: {}, body: {}",
                    number, status, response_body
                );
                Ok(false)
            }
        }
        Err(err) => {
            error!(
                "Failed to send error patch request for order {}: {}",
                number, err
            );
            Err(err)
        }
    }
}

async fn update_warehouse_order_status(
    number: usize,
    session_id: &str,
    current_time: &str,
    client: &ClientWithMiddleware,
) -> Result<bool, reqwest_middleware::Error> {
    let uri = format!("/b1s/v1/Orders({number})");

    let body = json!({
        // "U_Warehouse_Order": "Y",
        "U_Warehouse_Order": "N",
        "U_Warehouse_Order_Date": current_time,
        "U_Warehouse_Order_Process": "940 Generation",
    })
    .to_string();

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(
        COOKIE,
        HeaderValue::from_str(session_id).expect("Failed to create COOKIE header"),
    );

    let url = format!("{BASE_URL}{uri}");
    debug!("Patch Body to {url} for Order {number}: {body}");

    match send_request(client, Method::PATCH, url, Bytes::from(body), headers).await {
        Ok(response) => {
            let status = response.status();
            let response_body = response
                .text()
                .await
                .unwrap_or_else(|_| "Failed to read response body".to_string());

            if status.is_success() {
                debug!(
                    "Patch request successfully sent for order {}, status code {}",
                    number, status
                );
                Ok(true)
            } else {
                error!(
                    "Patch request failed for order {}, status: {}, body: {}",
                    number, status, response_body
                );
                Ok(false)
            }
        }
        Err(err) => {
            error!("Failed to send patch request for order {}: {}", number, err);
            Err(err)
        }
    }

    // let response = match client.patch(url).headers(headers).json(&body).send().await {
    /*
    for attempt in 1..=SEND_PATCH_ATTEMPTS {
        let url = format!("{BASE_URL}{uri}");
        info!("Patch Body to {url} for Order {number}: {body}");

        let response = match send_request(
            client,
            Method::PATCH,
            url,
            Bytes::from(body.clone()),
            headers.clone(),
        )
        .await
        {
            Ok(response) => {
                debug!("Patch Response for Order {number}: {:?}", response);
                response
            }
            Err(err) if attempt < SEND_PATCH_ATTEMPTS => {
                info!("Patch request attempt {attempt} for order {number} failed, error: {err}, retrying");
                sleep(Duration::from_secs(5 * attempt as u64)).await;
                continue;
            }
            Err(err) => {
                error!("Failed to send patch request, {err}");
                return Err(err);
            }
        };

        let status = response.status();
        let body_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Failed to read body".to_string());

        if status.is_success() {
            debug!(
                "Patch request successfully sent for order {}, status code {}",
                number, status
            );
            return Ok(true);
        }

        let error_msg = format!(
            "Patch request error for order {}, status: {}, body: {}",
            number, status, body_text
        );

        if attempt < SEND_PATCH_ATTEMPTS {
            error!("{} (attempt {}), retrying", error_msg, attempt);
        } else {
            error!("{}", error_msg);
            return Ok(false);
        }
    }

    Ok(true)
    */
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
) -> Result<Vec<Row>, anyhow::Error> {
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
            return Err(anyhow!(err));
        }
    };

    return match stream.into_first_result().await {
        Ok(rows) => {
            debug!("Query {query_str} returning {:?}", rows);
            Ok(rows)
        }
        Err(err) => {
            error!("Failed to fetch result for query {query_str}: {err}");
            return Err(anyhow!(err));
        }
    };
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

async fn alter_order_value(
    order: &mut Value,
    field_name: &str,
    new_value: &str,
    parent_field: &str,
) {
    // Will not assign empty string to JSON
    if !new_value.is_empty() {
        if parent_field.is_empty() {
            if let Some(name) = order.get_mut(field_name) {
                *name = json!(new_value);
            }
        } else if let Some(user_obj) = order.get_mut(parent_field).and_then(|u| u.as_object_mut()) {
            if let Some(name) = user_obj.get_mut(field_name) {
                *name = json!(new_value);
            }
        }
    }
}

async fn insert_order_value(order: &mut Value, field_name: &str, value: &str) {
    if let Some(ord) = order.as_object_mut() {
        ord.insert(field_name.to_string(), json!(value));
    }
}

async fn add_to_address_extension(order: &mut Value, field_name: &str, value: &str) {
    if let Some(ord) = order.as_object_mut() {
        // Ensure AddressExtension object exists
        let addr_ext = ord
            .entry("AddressExtension".to_string())
            .or_insert_with(|| json!({}));

        // Insert the field into AddressExtension
        if let Some(addr_obj) = addr_ext.as_object_mut() {
            addr_obj.insert(field_name.to_string(), json!(value));
        }
    }
}
// async fn add_to_address_extension(order: &mut Orders, field_name: &str, value: &str) {
//     // Ensure address_extension exists
//     if order.address_extension.is_none() {
//         order.address_extension = Some(AddressExtension::default());
//     }

//     if let Some(addr_ext) = &mut order.address_extension {
//         match field_name {
//             "BillToStreet" => addr_ext.bill_to_street = Some(value.to_string()),
//             "BillToCity" => addr_ext.ship_to_city = Some(value.to_string()),
//             "BillToBlock" => addr_ext.ship_to_state = Some(value.to_string()),
//             "BillToAddress2" => addr_ext.ship_to_zip_code = Some(value.to_string()),
//             "BillToZipCode" => addr_ext.ship_to_zip_code = Some(value.to_string()),
//             "BillToCountry" => addr_ext.ship_to_zip_code = Some(value.to_string()),
//             "BillToBlock" => addr_ext.ship_to_zip_code = Some(value.to_string()),
//             _ => println!("Unknown address extension field: {}", field_name),
//         }
//     }
// }
async fn insert_order_item_value(
    order: &mut Value,
    item_code: &str,
    line_num: &i32,
    field_name: &str,
    value: &str,
) {
    if let Some(ord) = order.as_object_mut() {
        // ord.insert(field_name.to_string(), json!(value));
        if let Some(lines) = ord.get_mut("DocumentLines").and_then(|v| v.as_array_mut()) {
            for line in lines.iter_mut() {
                if let (Some(line_num_val), Some(item_code_val)) =
                    (line.get("LineNum"), line.get("ItemCode"))
                {
                    if line_num_val == &json!(line_num) && item_code_val == &json!(item_code) {
                        line[field_name] = json!(value);
                    }
                }
            }
        }
    }
}

fn normalize_numbers(value: Value) -> Value {
    match value {
        Value::Number(n) => {
            // Convert to string explicitly
            Value::String(n.to_string())
        }
        Value::Array(arr) => Value::Array(arr.into_iter().map(normalize_numbers).collect()),
        Value::Object(map) => Value::Object(
            map.into_iter()
                .map(|(k, v)| (k, normalize_numbers(v)))
                .collect(),
        ),
        other => other,
    }
}
