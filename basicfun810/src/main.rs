// prism update

mod order_structure;
mod prism_structure;
use prism_structure::{Agency, Output};
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
use std::borrow::Cow;
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::path::{Path, PathBuf};
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
    output_dir: PathBuf,

    #[arg(short, long)]
    archive_dir: PathBuf,

    #[arg(short, long)]
    process_id: String,

    #[arg(short, long)]
    error_dir: PathBuf,

    #[arg(short, long)]
    logs_dir: PathBuf,

    #[arg(long)]
    prism_path: PathBuf,

    #[arg(long)]
    process_data_path: PathBuf,
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


// fn fetch_prism_data(process_data_path: &Path) -> Result<HashMap<String, String>> {
//     let mut file_map = HashMap::new();
//     for entry in std::fs::read_dir(process_data_path)? {
//         let entry = entry?;
//         let path = entry.path();
//         if path.is_file() {
//             let contents = std::fs::read_to_string(&path)?;
//             let output: Output = match quick_xml::de::from_str(&contents) {
//                 Ok(o) => o,
//                 Err(err) => {
//                     error!("Failed to parse prism data {}: {err}", path.display());
//                     continue;
//                 }
//             };
//             if let Some(erp) = output.erp_file_name {
//                 let name = Path::new(&erp.into_owned())
//                     .file_name()
//                     .unwrap()
//                     .to_string_lossy()
//                     .to_string();
//                 file_map.insert(name, path.to_string_lossy().to_string());
//             }
//             if let Some(edi) = output.edi_file_name {
//                 let name = Path::new(&edi.into_owned())
//                     .file_name()
//                     .unwrap()
//                     .to_string_lossy()
//                     .to_string();
//                 file_map.insert(name, path.to_string_lossy().to_string());
//             }
//         }
//     }
//     info!("Loaded {} Prism process data files", file_map.len());
//     Ok(file_map)
// }

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

    let url = format!("{}/b1s/v1/Login", base_url);

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

async fn get_invoices(
    base_url: &str,
    session_id: &str,
    client: &ClientWithMiddleware,
    output_dir: &std::path::Path,
    sap_pool: &Arc<Pool<bb8_tiberius::ConnectionManager>>,
    prism_path: &Path,
    _process_id: &str,
    _company: &str,
    // file_hashmap: &HashMap<String, String>,
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
                        debug!("Successfully parsed invoices response JSON: {:#?}", data);
                        data.orders
                    },
                    Err(err) => {
                        error!("Failed to parse invoices response JSON: {err}");
                        return Err(anyhow!("Failed to parse invoices response JSON: {err}"));
                    }
                }
             } else {
                error!("Failed to get invoices, status {}", response.status());
                return Err(anyhow!("Invoices request failed with status {}", response.status()));
            }
        },
        Err(err) => {
            error!("Failed to send request for invoices: {err}");
            return Err(anyhow!("Failed to send request for invoices: {err}"));
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
            let card_code = order.get("CardCode").and_then(|v| v.as_str()).unwrap_or("Unknown");
            let filename = format!("{}-810_{}-{}.xml", card_code, doc_num, now.format("%Y%m%d%H%M%S"));
            // let prism_file = file_hashmap.get(&filename).cloned();
            let path = output_dir.join(&filename);
            // if let Some(doc_entry) = order.get("DocEntry").and_then(|value| value.as_u64()) {
            //     match get_tracking_by_doc_entry(sap_pool, doc_entry).await {
            //         Ok(tracking_items) if !tracking_items.is_empty() => {
            //             if let Some(obj) = order.as_object_mut() {
            //                 let tracking_value = serde_json::to_value(&tracking_items)
            //                     .map_err(|e| anyhow!("Failed to serialize tracking data: {e}"))?;
            //                 obj.insert("Tracking".to_string(), tracking_value);
            //             }
            //         }
            //         Ok(_) => {
            //             debug!("No tracking rows found for DocEntry {}", doc_entry);
            //         }
            //         Err(err) => {
            //             warn!("Failed to load tracking for DocEntry {}: {}", doc_entry, err);
            //         }
            //     }
            // }

            let doc_entry = order.get("DocEntry").cloned();
            let doc_lines: Vec<Value> = order
                .get("DocumentLines")
                .and_then(|v| v.as_array())
                .map(|lines| {
                    lines
                        .iter()
                        .map(|line| {
                            let base_line = line.get("LineNum").and_then(|v| v.as_i64()).unwrap_or(0);
                            json!({
                                "BaseType": 15,
                                "BaseEntry": doc_entry.clone().unwrap_or(Value::Null),
                                "BaseLine": base_line
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();

            fn trim_sap_date(val: Option<&Value>) -> Value {
                match val.and_then(|v| v.as_str()) {
                    Some(s) => Value::String(s.chars().take(10).collect()),
                    None => Value::Null,
                }
            }

            let extracted = json!({
                "DocDate": trim_sap_date(order.get("DocDate")),
                "DocDueDate": trim_sap_date(order.get("DocDueDate")),
                "CardCode": order.get("CardCode"),
                "Series": 82,
                "TaxDate": trim_sap_date(order.get("TaxDate")),
                "NumAtCard": order.get("NumAtCard"),
                "SalesPersonCode": order.get("SalesPersonCode"),
                "TransportationCode": order.get("TransportationCode"),
                "BPL_IDAssignedToInvoice": order.get("BPL_IDAssignedToInvoice"),
                "U_TBD_SI_Remarks": order.get("U_TBD_SI_Remarks"),
                "U_TBD_SA_Remarks": order.get("U_TBD_SA_Remarks"),
                "DocumentLines": doc_lines,
                "DocumentAdditionalExpenses": order
                    .get("DocumentAdditionalExpenses")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter().map(|exp| {
                            json!({
                                "ExpenseCode": exp.get("ExpenseCode"),
                                "LineTotal": exp.get("LineTotal"),
                                "TaxCode": exp.get("TaxCode"),
                                "BaseDocEntry": order.get("DocEntry"),
                                "BaseDocLine": exp.get("BaseDocLine"),
                                "BaseDocType": 15,
                                "BaseDocumentReference": order.get("DocNum"),
                            })
                        }).collect::<Vec<_>>()
                    })
                    .unwrap_or_default(),
                "AddressExtension": order.get("AddressExtension"),
            });

            let post_body = serde_json::to_vec(&extracted)?;
            info!("extracted: {:?}", extracted);
            info!("post_body: {}", String::from_utf8_lossy(&post_body));
            let mut post_headers = HeaderMap::new();
            post_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
            post_headers.insert(
                COOKIE,
                HeaderValue::from_str(session_id).expect("Failed to create COOKIE header"),
            );
            let invoices_url = format!("{base_url}/Invoices");

            let body_text = match send_request(client, Method::POST, invoices_url, Bytes::from(post_body), post_headers).await {
                Ok(post_resp) => {
                    let status = post_resp.status();
                    let text = post_resp.text().await.unwrap_or_default();
                    info!("Invoices POST response [{}]: {}", status, text);
                    if !status.is_success() {
                        error!("Invoice POST failed with status {}, skipping", status);
                        // if let Some(ref prism_file) = prism_file {
                        //     match std::fs::read_to_string(prism_file) {
                        //         Ok(contents) => {
                        //             if let Ok(mut process_data) = quick_xml::de::from_str::<Output>(&contents) {
                        //                 process_data.error_type = Some(Cow::Borrowed("ERP"));
                        //                 process_data.agency = Agency::ERROR;
                        //                 process_data.error_description = Some(Cow::from(format!("Invoice POST failed: status {}", status)));
                        //                 process_data.status = Cow::Borrowed("ERROR");
                        //                 process_data.plant_name = Some(Cow::Borrowed("BasicFun"));
                        //                 let mut xml_string = String::new();
                        //                 if let Ok(ser) = quick_xml::se::Serializer::with_root(&mut xml_string, Some("Output")) {
                        //                     let _ = process_data.serialize(ser);
                        //                 }
                        //                 let prism_dest = prism_path.join(Path::new(prism_file).file_name().unwrap());
                        //                 if let Err(err) = tokio::fs::write(&prism_dest, &xml_string).await {
                        //                     error!("Failed to write prism error file: {err}");
                        //                 }
                        //             }
                        //         }
                        //         Err(err) => error!("Failed to read prism file {prism_file}: {err}"),
                        //     }
                        // }
                        continue;
                    }
                    text
                }
                Err(err) => {
                    error!("Failed to POST to Invoices: {err}, skipping");
                    // if let Some(ref prism_file) = prism_file {
                    //     match std::fs::read_to_string(prism_file) {
                    //         Ok(contents) => {
                    //             if let Ok(mut process_data) = quick_xml::de::from_str::<Output>(&contents) {
                    //                 process_data.error_type = Some(Cow::Borrowed("ERP"));
                    //                 process_data.agency = Agency::ERROR;
                    //                 process_data.error_description = Some(Cow::from(format!("Invoice POST request failed: {err}")));
                    //                 process_data.status = Cow::Borrowed("ERROR");
                    //                 process_data.plant_name = Some(Cow::Borrowed("BasicFun"));
                    //                 let mut xml_string = String::new();
                    //                 if let Ok(ser) = quick_xml::se::Serializer::with_root(&mut xml_string, Some("Output")) {
                    //                     let _ = process_data.serialize(ser);
                    //                 }
                    //                 let prism_dest = prism_path.join(Path::new(prism_file).file_name().unwrap());
                    //                 if let Err(err) = tokio::fs::write(&prism_dest, &xml_string).await {
                    //                     error!("Failed to write prism error file: {err}");
                    //                 }
                    //             }
                    //         }
                    //         Err(read_err) => error!("Failed to read prism file {prism_file}: {read_err}"),
                    //     }
                    // }
                    continue;
                }
            };

            // adding some fields to the invoice JSON for XML output
            let card_code = order.get("CardCode").and_then(|v| v.as_str()).unwrap_or("");
            let query = format!(
                "SELECT ExtraDays, VolumDscnt FROM OCTG T0 INNER JOIN OCRD T1 ON T0.GroupNum = T1.GroupNum WHERE T1.CardCode = '{}'",
                card_code
            );
            let (extra_days, volum_dscnt) = match send_query(sap_pool, &query, &[]).await {
                Ok(rows) => {
                    if let Some(row) = rows.first() {
                        let ed = row.get::<i16, _>("ExtraDays").unwrap_or(0) as i32;
                        let vd = row.try_get::<Numeric, _>("VolumDscnt").ok().flatten().map(|v| v.to_string()).unwrap_or_else(|| "0".to_string());
                        (ed, vd)
                    } else {
                        (0, "0".to_string())
                    }
                }
                Err(err) => {
                    error!("Failed to query OCTG/OCRD: {err}");
                    (0, "0".to_string())
                }
            };

            let mut invoice_value: Value = serde_json::from_str(&body_text).unwrap_or(Value::Null);
            insert_order_value(&mut invoice_value, "U_TermDiscountPercent", &volum_dscnt.to_string()).await;
            insert_order_value(&mut invoice_value, "U_TermsNetDays", &extra_days.to_string()).await;
            insert_order_value(
                &mut invoice_value,
                "CustomerPONUM",
                order.get("NumAtCard").and_then(|v| v.as_str()).unwrap_or(""),
            ).await;
            insert_order_value(
                &mut invoice_value,
                "CustomerREFOQvalue",
                order.get("U_OQ_REF_VALUE").and_then(|v| v.as_str()).unwrap_or(""),
            ).await;

            let mut order_xml = to_string_with_root("root", &invoice_value).expect("Failed to serialize to XML");
            order_xml = order_xml
                .replace("\r\n ", " ")
                .replace("\r\n", " ")
                .replace("\r ", " ")
                .replace("\n ", " ")
                .replace(['\r', '\n'], " ");

            tokio::fs::write(&path, order_xml).await.map_err(|e| anyhow!("Unable to write XML file: {e}"))?;
            info!("Saved invoices XML to {}", path.display());

            // if let Some(ref prism_file) = prism_file {
            //     let prism_dest = prism_path.join(Path::new(prism_file).file_name().unwrap());
            //     if let Err(err) = tokio::fs::rename(prism_file, &prism_dest).await {
            //         error!("Failed to move prism file {prism_file}: {err}");
            //     }
            // }

            let update_query = format!("UPDATE ODLN SET U_945_Advice = 'N' WHERE DocNum = {}", doc_num);
            match send_query(sap_pool, &update_query, &[]).await {
                Ok(_) => info!("Updated U_945_Advice to 'N' for DocNum {}", doc_num),
                Err(err) => error!("Failed to update U_945_Advice for DocNum {}: {}", doc_num, err),
            }
        }
    }

    // info!("Saved invoices XML to {}", xml_path.display());
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
    dotenv().ok();

    // Clap Args
    let mut args = Args::parse();

    // configuring logs
    std::fs::create_dir_all(&args.logs_dir)?;
    let now = Local::now();
    let log_file_name = format!("logs810_{}.log", now.format("%Y%m%d_%H%M%S"));
    let log_file = std::fs::File::create(args.logs_dir.join(&log_file_name))?;
    let (non_blocking, _guard) = tracing_appender::non_blocking(log_file);

    tracing_subscriber::fmt()
        .with_ansi(false)
        .with_writer(non_blocking)
        .with_env_filter(EnvFilter::new("info,tiberius=warn"))
        .init();

    let output_dir = Arc::new(args.output_dir);
    let now = Local::now();
    args.archive_dir.push(now.format("%Y/%m/%d").to_string());
    let pre_archive_dir = Arc::new(args.archive_dir);
    let error_dir = args.error_dir;

    std::fs::create_dir_all(&args.prism_path)?;
    std::fs::create_dir_all(&args.process_data_path)?;
    // let file_hashmap = fetch_prism_data(&args.process_data_path)?;

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
        .retry_bounds(Duration::from_millis(200), Duration::from_secs(1))
        .build_with_max_retries(2);

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

    // Process invoices using DB query
    let dn_resp = get_invoices(&base_url, &session_id, &*client, &*output_dir, &sap_pool, &args.prism_path, &args.process_id, &company).await?;
    let count = dn_resp.get("value").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0);
    info!("Retrieved {} invoices", count);

    println!("\nAll files processed.");
    info!("Processing Time {:?}", start.elapsed());
    Ok(())
}

async fn insert_order_value(order: &mut Value, field_name: &str, value: &str) {
    if let Some(ord) = order.as_object_mut() {
        ord.insert(field_name.to_string(), json!(value));
    }
}

// basicfun810.exe --output-dir "C:\Users\BasicFun\Desktop\810\output" --archive-dir "C:\Users\BasicFun\Desktop\810\output" --process-id "1" --error-dir "C:\Users\BasicFun\Desktop\810\error" --logs-dir "C:\Users\BasicFun\Desktop\810\logs" --prism-path "C:\Users\BasicFun\Desktop\810\prism" --process-data-path "C:\Users\BasicFun\Desktop\810\process_data"
