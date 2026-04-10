use dotenv::dotenv;
use reqwest::blocking::Client;
use reqwest::header::CONTENT_TYPE;
use serde::Deserialize;
use std::env;
use std::fs;
use std::path::Path;
use serde_json::Value;
use std::path::PathBuf;

// const BASE_URL: &str = "https://BFAZWSAP01.corp.basicfun.com:50000";
const BASE_URL: &str = "http://127.0.0.1:8000/";

// -----------------------------
// LOGIN RESPONSE STRUCT
// -----------------------------
#[derive(Deserialize, Debug)]
struct LoginResponse {
    #[serde(rename = "SessionTimeout")]
    session_timeout: i32,
}

// -----------------------------
// SAP CLIENT STRUCT
// -----------------------------
struct SapClient {
    client: Client,
    session_id: Option<String>,
    session_timeout: Option<i32>,
}

impl SapClient {
    fn new() -> Self {
        let client = Client::builder()
            .cookie_store(true)
            .build()
            .unwrap();

        Self {
            client,
            session_id: None,
            session_timeout: None,
        }
    }

    // -----------------------------
    // LOGIN FUNCTION
    // -----------------------------
    fn login(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        dotenv().ok();

        let company_db = env::var("CompanyDB")?;
        let username = env::var("UserName")?;
        let password = env::var("Password")?;

        let url = format!("{}/Login", BASE_URL);

        let payload = serde_json::json!({
            "CompanyDB": company_db,
            "UserName": username,
            "Password": password
        });

        let response = self
            .client
            .post(&url)
            .header(CONTENT_TYPE, "application/json")
            .json(&payload)
            .send()?;

        if response.status().is_success() {
            let resp_json: LoginResponse = response.json()?;

            self.session_id = Some("active".to_string());
            self.session_timeout = Some(resp_json.session_timeout);

            println!("✅ Login successful");
            println!("⏱ Session timeout: {} seconds", resp_json.session_timeout);

            Ok(())
        } else {
            Err(format!("❌ Login failed: {}", response.text()?).into())
        }
    }

    // -----------------------------
    // SESSION CHECK
    // -----------------------------
    fn is_session_expired(&self) -> bool {
        self.session_id.is_none()
    }

    // -----------------------------
    // SECOND FUNCTION (API CALL)
    // -----------------------------
    fn get_st(&mut self, data: &str) -> Result<String, Box<dyn std::error::Error>> {
        if self.is_session_expired() {
            self.login()?;
        }

        let parts: Vec<&str> = data.split(',').collect();

        let url = format!(
            "{}/$crossjoin(BusinessPartners,BusinessPartners/BPAddresses)\
            ?$expand=BusinessPartners($select=CardType,CardCode),\
            BusinessPartners/BPAddresses($select=AddressName,AddressType,GlobalLocationNumber)\
            &$filter=BusinessPartners/CardCode eq BusinessPartners/BPAddresses/BPCode \
            and BusinessPartners/CardCode eq '{}' \
            and BusinessPartners/BPAddresses/GlobalLocationNumber eq '{}' \
            and BusinessPartners/CardType eq '{}' \
            and BusinessPartners/BPAddresses/AddressType eq 'bo_ShipTo' \
            &$top=1",
            BASE_URL,
            parts[3],
            parts[1],
            parts[0]
        );

        let response = self.client.get(&url).send()?;

        if !response.status().is_success() {
            return Err(format!("❌ API failed: {}", response.text()?).into());
        }

        let json: serde_json::Value = response.json()?;

        if let Some(arr) = json.get("value").and_then(|v| v.as_array()) {
            if let Some(first) = arr.first() {
                if let Some(address) = first
                    // .get("BusinessPartners/BPAddresses")
                    .get("sap-test")
                    .and_then(|bp| bp.get("AddressName"))
                    .and_then(|a| a.as_str())
                {
                    return Ok(address.to_string());
                }
            }
        }

        Ok(data.to_string())
    }
}


fn read_files_from_directory(dir_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let entries = fs::read_dir(dir_path)?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            let file_name = path.file_name().unwrap().to_string_lossy();
            let content = fs::read_to_string(&path)?;

            println!("=== {} ===", file_name);
            println!("{}", content);
            println!();
        }
    }

    Ok(())
}

fn main() {

    match read_files_from_directory(dir_path) {
        Ok(_) => println!("Done reading files."),
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

// -----------------------------
// MAIN
// -----------------------------
fn main() -> Result<(), Box<dyn std::error::Error>> {

    // -------------collect args from console-------------
    let args: Vec<String> = env::args().collect();

    if args.len() < 4 {
        eprintln!("Usage: {} <directory_path>", args[0]);
        std::process::exit(1);
    }
    println!("{:?}, lenght: {}", args, args.len());

    let input_dir = &args[1];
    let output_dir = &args[2];
    let error_dir = &args[3];
    println!("{}, {}, {}", input_dir, output_dir, error_dir);
    // -------------------------------------------

    let mut client = SapClient::new();

    let input_path = Path::new(input_dir);
    let file_content = fs::read_to_string(input_path)?;

    let mut json: Value = serde_json::from_str(&file_content)?;

    let ship_to_value = json["ship_to"]
        .as_str()
        .ok_or("ship_to field missing or invalid")?;


    let updated_ship_to = client.get_st(ship_to_value)?;

    // -----------------------------
    // UPDATE JSON
    // -----------------------------
    json["ship_to"] = Value::String(updated_ship_to);

    // -----------------------------
    // WRITE OUTPUT FILE
    // -----------------------------
    let file_name = input_path
        .file_name()
        .ok_or("❌ Invalid input file name")?;

    let output_path = Path::new(output_dir).join(file_name);

    fs::write(output_path, serde_json::to_string_pretty(&json)?)?;

    println!("✅ File processed successfully");

    Ok(())
}