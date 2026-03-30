use std::collections::HashMap;
use std::fs::{File, create_dir_all};
use std::io::BufReader;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;

use dotenv::dotenv;
use reqwest::Client;
use serde_json::{Value, json};

// ----------------------
// SESSION STRUCT
// ----------------------
#[derive(Clone, Debug)]
struct Session {
    session_id: String,
    expires_at: u64,
    cookies: HashMap<String, String>,
}

impl Session {
    fn is_expired(&self) -> bool {
        let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
        now >= self.expires_at
    }
}

// ----------------------
// SAP CLIENT
// ----------------------
#[derive(Clone)]
struct SapClient {
    client: Client,
    session: Arc<Mutex<Option<Session>>>,
}

impl SapClient {
    fn new() -> Self {
        Self {
            client: Client::builder()
                .cookie_store(true)
                .build()
                .unwrap(),
            session: Arc::new(Mutex::new(None)),
        }
    }

    async fn login(&self) -> Result<Session, String> {
        let company_db = std::env::var("CompanyDB").unwrap_or_default();
        let username = std::env::var("UserName").unwrap_or_default();
        let password = std::env::var("Password").unwrap_or_default();

        let mut data = HashMap::new();
        data.insert("CompanyDB".to_string(), company_db);
        data.insert("UserName".to_string(), username);
        data.insert("Password".to_string(), password);

        let url = "https://f08sl.softengineapps.com:50000/b1s/v1/Login";

        let resp = self
        .client
        .post(url)
        .json(&data)
        .send()
        .await
        .map_err(|e| e.to_string())?;

        if resp.status().is_success() {
            let mut session_id = String::new();
            let mut cookies_map = HashMap::new();

            for cookie in resp.cookies() {
                let name = cookie.name().to_string();
                let value = cookie.value().to_string();
                cookies_map.insert(name.clone(), value.clone());
                if name == "B1SESSION" {
                    session_id = value;
                }
            }

            let json: Value = resp.json().await.map_err(|e| e.to_string())?;
            let session_timeout = json["SessionTimeout"].as_u64().unwrap_or(0) * 60;
            let expires_at = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() + session_timeout;

            Ok(Session { session_id, expires_at, cookies: cookies_map })
        } else {
            let json: Value = resp.json().await.unwrap_or_default();
            let error_msg = json["error"]["message"]["value"].as_str().unwrap_or("Invalid credentials").to_string();
            Err(error_msg)
        }
    }

    async fn get_session(&self) -> Result<Session, String> {
        let mut lock = self.session.lock().await;
        match &*lock {
            Some(session) if !session.is_expired() => Ok(session.clone()),
            _ => {
                let new_session = self.login().await?;
                *lock = Some(new_session.clone());
                Ok(new_session)
            }
        }
    }

    // ----------------------
    // Get ShipTo field
    // ----------------------
    pub async fn get_shipto(&self, card_type: &str, gln: &str, card_code: &str) -> Result<String, String> {
        let session = self.get_session().await?;

        let url = format!(
            "https://f08sl.softengineapps.com:50000/b1s/v1/$crossjoin(BusinessPartners,BusinessPartners/BPAddresses)?\
            $expand=BusinessPartners($select=CardType,CardCode),\
            BusinessPartners/BPAddresses($select=AddressName,AddressType,GlobalLocationNumber)&\
            $filter=BusinessPartners/CardCode eq BusinessPartners/BPAddresses/BPCode \
            and BusinessPartners/CardCode eq '{}' \
            and BusinessPartners/BPAddresses/GlobalLocationNumber eq '{}' \
            and BusinessPartners/CardType eq '{}' \
            and BusinessPartners/BPAddresses/AddressType eq 'bo_ShipTo' &$top=1",
            card_code, gln, card_type
        );

        let resp = self.client
            .get(&url)
            .header("Cookie", format!("B1SESSION={}", session.session_id))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if resp.status().is_success() {
            let json: Value = resp.json().await.map_err(|e| e.to_string())?;
            if let Some(values) = json.get("value").and_then(|v| v.as_array()) {
                if !values.is_empty() {
                    if let Some(address) = values[0].get("BusinessPartners/BPAddresses").and_then(|v| v.get("AddressName")).and_then(|v| v.as_str()) {
                        return Ok(address.to_string());
                    }
                }
            }
            Ok("".to_string())
        } else {
            Err(format!("SAP request failed: {}", resp.status()))
        }
    }
}

// ----------------------
// PROCESS JSON INPUT/OUTPUT
// ----------------------
async fn process_json(input_path: &str, output_path: &str, sap_client: &SapClient) -> Result<(), Box<dyn std::error::Error>> {
    // Ensure output folder exists
    if let Some(parent) = std::path::Path::new(output_path).parent() {
        create_dir_all(parent)?;
    }

    // Load JSON file
    let file = File::open(input_path)?;
    let reader = BufReader::new(file);
    let mut data: Value = serde_json::from_reader(reader)?;

    if let Some(array) = data.as_array_mut() {
        for obj in array.iter_mut() {
            let card_type = obj.get("CardType").and_then(|v| v.as_str()).unwrap_or("");
            let gln = obj.get("GlobalLocationNumber").and_then(|v| v.as_str()).unwrap_or("");
            let card_code = obj.get("CardCode").and_then(|v| v.as_str()).unwrap_or("");

            let shipto = sap_client.get_shipto(card_type, gln, card_code).await.unwrap_or_default();
            obj["ShipTo"] = json!(shipto);
        }
    }

    // Write updated JSON
    let mut output_file = File::create(output_path)?;
    serde_json::to_writer_pretty(&mut output_file, &data)?;

    Ok(())
}

// ----------------------
// MAIN
// ----------------------
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let sap_client = SapClient::new();

    process_json(
        "input/input.json",
        "output/output.json",
        &sap_client
    ).await?;

    println!("✅ JSON processing completed successfully!");
    Ok(())
}