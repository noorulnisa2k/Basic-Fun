use std::collections::HashMap;
// use std::fs::{File, create_dir_all};
// use std::io::BufReader;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;

use std::env;
use dotenv::dotenv;
use reqwest::Client;
// use serde_json::{Value, json};
use serde_json::Value;


const BASE_URL: &str = "https://BFAZWSAP01.corp.basicfun.com:50000";
// const THREADS: usize = 8;

// ----------------------
// SESSION STRUCT
// ----------------------
#[derive(Clone, Debug)]
struct Session {
    session_id: String,
    expires_at: u64, // Unix timestamp
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
// SAP CLIENT STRUCT
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
            .cookie_store(true) // requires "cookie_store" feature in Cargo.toml
            .build()
            .unwrap(),
            session: Arc::new(Mutex::new(None)),
        }
    }

    // ----------------------
    // LOGIN FUNCTION
    // ----------------------
    async fn login(&self) -> Result<Session, String> {
        let company_db = std::env::var("CompanyDB").unwrap_or_default();
        let username = std::env::var("UserName").unwrap_or_default();
        let password = std::env::var("Password").unwrap_or_default();

        let mut data = HashMap::new();
        data.insert("CompanyDB".to_string(), company_db);
        data.insert("UserName".to_string(), username);
        data.insert("Password".to_string(), password);

        let url = format!("{}/b1s/v1/Login",BASE_URL);

        let resp = self
            .client
            .post(url)
            .json(&data)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if resp.status().is_success() {
            // Extract session cookie
            let mut session_id = String::new();
            let mut cookies_map = HashMap::new();
            for cookie in resp.cookies() {
                if cookie.name() == "B1SESSION" {
                    session_id = cookie.value().to_string();
                    cookies_map.insert("B1SESSION".to_string(), session_id.clone());
                }
            }

            // Get session timeout
            let json: Value = resp.json().await.map_err(|e| e.to_string())?;
            let session_timeout = json["SessionTimeout"].as_u64().unwrap_or(0) * 60;

            let expires_at = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                + session_timeout;

            Ok(Session {
                session_id,
                expires_at,
                cookies: cookies_map,
            })
        } else {
            let json: Value = resp.json().await.unwrap_or_default();
            let error_msg = json["error"]["message"]["value"]
                .as_str()
                .unwrap_or("Invalid credentials")
                .to_string();
            Err(error_msg)
        }
    }

    // ----------------------
    // GET SESSION (auto-login if expired)
    // ----------------------
    async fn get_session(&self) -> Result<Session, String> {
        let mut lock = self.session.lock().await;

        match &*lock {
            Some(session) if !session.is_expired() => Ok(session.clone()),
            _ => {
                // Session missing or expired → login
                let new_session = self.login().await?;
                *lock = Some(new_session.clone());
                Ok(new_session)
            }
        }
    }

    // ----------------------
    // EXAMPLE API CALL USING SESSION
    // ----------------------
    async fn example_call(&self) -> Result<(), String> {
        let session = self.get_session().await?;

        let url = "https://f08sl.softengineapps.com:50000/b1s/v1/SomeEndpoint";

        let resp = self
            .client
            .get(url)
            .header("Cookie", format!("B1SESSION={}", session.session_id))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if resp.status().is_success() {
            println!("✅ API call successful");
            Ok(())
        } else {
            Err(format!("API call failed: {}", resp.status()))
        }
    }
}


// ----------------------
// MAIN FUNCTION
// ----------------------
#[tokio::main]
async fn main() {
    dotenv().ok();

    // -------------collect args from console-------------
    let args: Vec<String> = env::args().collect();
    println!("{:?}, lenght: {}", args, args.len());

    let input_dir = &args[1];
    let output_dir = &args[2];
    let error_dir = &args[3];
    println!("{}, {}, {}", input_dir, output_dir, error_dir);
    // -------------------------------------------

    let sap_client = SapClient::new();

    match sap_client.get_session().await {
        Ok(session) => {
            println!("Session ID: {}", session.session_id);
            println!("Expires at: {}", session.expires_at);
            println!("Cookies: {:?}", session.cookies);
        }
        Err(err) => println!("Login failed: {}", err),
    }

    // Example call
    if let Err(err) = sap_client.example_call().await {
        println!("{}", err);
    }
}