mod order_structure_test;
use std::env;
use std::fs;
use std::path::Path;
use serde_json::Value;
use std::time::{Duration, Instant};

use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;

use order_structure_test::Orders;

#[derive(Serialize)]
struct LoginRequest {
    #[serde(rename = "CompanyDB")]
    company_db: String,
    #[serde(rename = "UserName")]
    user_name: String,
    #[serde(rename = "Password")]
    password: String,
}

#[derive(Deserialize, Debug)]
struct LoginResponse {
    #[serde(rename = "SessionId")]
    session_id: String,
}

#[derive(Deserialize, Debug)]
struct BPAddressEntry {
    #[serde(rename = "BusinessPartners/BPAddresses")]
    bp_addresses: BPAddressFields,
}

#[derive(Deserialize, Debug)]
struct BPAddressFields {
    #[serde(rename = "AddressName")]
    address_name: String,
}

#[derive(Deserialize, Debug)]
struct BPAddressResponse {
    value: Vec<BPAddressEntry>,
}

#[derive(Deserialize, Debug)]
struct CreateOrderResponse {
    #[serde(rename = "DocNum")]
    doc_num: i64,
    #[serde(rename = "DocEntry")]
    doc_entry: i64,
}

#[derive(Debug)]
struct SapSession {
    client: Client,
    cookies: String,
    created_at: Instant,
    expiry_duration: Duration,
}

impl SapSession {
    fn is_expired(&self) -> bool {
        self.created_at.elapsed() >= self.expiry_duration
    }

    fn time_remaining(&self) -> Duration {
        let elapsed = self.created_at.elapsed();
        if elapsed >= self.expiry_duration {
            Duration::ZERO
        } else {
            self.expiry_duration - elapsed
        }
    }
}

fn sap_login() -> Result<SapSession, Box<dyn Error>> {
    dotenv::dotenv().ok();

    let base_url = env::var("BASE_URL").expect("BASE_URL must be set");
    let company = env::var("Company_DB").expect("Company_DB must be set");
    let username = env::var("User_Name").expect("User_Name must be set");
    let password = env::var("Password").expect("Password must be set");

    let expiry_minutes: u64 = env::var("SESSION_EXPIRY_MINUTES")
        .unwrap_or_else(|_| "28".to_string())
        .parse()
        .unwrap_or(28);

    let client = Client::builder()
        .danger_accept_invalid_certs(true)
        .build()?;

    let url = format!("{}/Login", base_url);

    let payload = LoginRequest {
        company_db: company,
        user_name: username,
        password: password,
    };

    println!("--- Login Attempt ---");
    println!("URL: {}", url);
    match serde_json::to_string_pretty(&payload) {
        Ok(json) => println!("Body:\n{}", json),
        Err(e) => println!("Failed to serialize payload: {}", e),
    }
    println!("Session expiry set to {} minutes", expiry_minutes);
    println!("------------------");

    let response = client.post(&url).json(&payload).send()?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().unwrap_or_else(|_| "Could not read error body".to_string());

        return Err(format!("Login failed: {}. Details: {}", status, error_text).into());
    }

    let cookies = response
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .map(|v| v.split(';').next().unwrap_or("").trim().to_string())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("; ");

    let login_data: LoginResponse = response.json()?;
    println!("Login successful. SessionId: {}", login_data.session_id);
    println!("---------------------");

    Ok(SapSession {
        client,
        cookies,
        created_at: Instant::now(),
        expiry_duration: Duration::from_secs(expiry_minutes * 60),
    })
}

fn ensure_session(session: &mut SapSession) -> Result<(), Box<dyn Error>> {
    if session.is_expired() {
        println!(
            "Session expired (age: {:.1}s). Re-authenticating...",
            session.created_at.elapsed().as_secs_f64()
        );
        *session = sap_login()?;
    } else {
        println!(
            "Session valid. ~{:.0}s remaining.",
            session.time_remaining().as_secs_f64()
        );
    }
    Ok(())
}

fn get_bp_address(
    session:   &mut SapSession,
    card_code: &str,
    gln:       &str,
    card_type: &str,
) -> Result<Option<String>, Box<dyn Error>> {
    ensure_session(session)?;

    let base_url = env::var("BASE_URL")?;

    let url = format!(
        "{}/$crossjoin(BusinessPartners,BusinessPartners/BPAddresses)\
        ?$expand=BusinessPartners($select=CardType,CardCode),\
        BusinessPartners/BPAddresses($select=AddressName,AddressType,GlobalLocationNumber)\
        &$filter=BusinessPartners/CardCode eq BusinessPartners/BPAddresses/BPCode \
        and BusinessPartners/CardCode eq '{}' \
        and BusinessPartners/BPAddresses/GlobalLocationNumber eq '{}' \
        and BusinessPartners/CardType eq '{}' \
        and BusinessPartners/BPAddresses/AddressType eq 'bo_ShipTo'",
        base_url, card_code, gln, card_type
    );

    println!("Request URL:\n{}", url);

    let response = session
        .client
        .get(&url)
        .header("Cookie", &session.cookies)
        .send()?;

    let response = if response.status() == 401 {
        println!("Received 401. Re-authenticating and retrying...");
        *session = sap_login()?;

        let retry = session
            .client
            .get(&url)
            .header("Cookie", &session.cookies)
            .send()?;

        if !retry.status().is_success() {
            let status = retry.status();
            let body = retry.text().unwrap_or_default();
            return Err(format!("Retry failed after re-auth: {} - {}", status, body).into());
        }
        retry
    } else {
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(format!("Request failed: {} - {}", status, body).into());
        }
        response
    };

    let parsed: BPAddressResponse = response.json()?;

    let ship_to = parsed
        .value
        .into_iter()
        .next()
        .map(|entry| entry.bp_addresses.address_name);

    Ok(ship_to)
}

fn create_order(
    session: &mut SapSession,
    order: &Orders,
) -> Result<CreateOrderResponse, Box<dyn Error>> {
    ensure_session(session)?;

    let base_url = env::var("BASE_URL")?;
    let url = format!("{}/Orders", base_url);

    println!("--- Creating Order ---");
    println!("URL: {}", url);
    println!("Payload:\n{}", serde_json::to_string_pretty(&order)?);

    let response = session
        .client
        .post(&url)
        .header("Cookie", &session.cookies)
        .json(&order)
        .send()?;

    let response = if response.status() == 401 {
        println!("Received 401. Re-authenticating...");
        *session = sap_login()?;

        let retry = session
            .client
            .post(&url)
            .header("Cookie", &session.cookies)
            .json(&order)
            .send()?;

        if !retry.status().is_success() {
            let status = retry.status();
            let body = retry.text().unwrap_or_default();
            return Err(format!("Order creation failed after re-auth: {} - {}", status, body).into());
        }
        retry
    } else {
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(format!("Order creation failed: {} - {}", status, body).into());
        }
        response
    };

    let created: CreateOrderResponse = response.json()?;
    println!(
        "✅ Order created — DocNum: {}, DocEntry: {}",
        created.doc_num, created.doc_entry
    );

    Ok(created)
}

fn collect_files(dir: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    if !dir.exists() || !dir.is_dir() {
        return Err(format!("Invalid directory: {}", dir.display()).into());
    }

    let files = fs::read_dir(dir)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .map(|path| path.display().to_string())
        .collect();

    Ok(files)
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 4 {
        eprintln!("Usage: {} <input_dir> <output_dir> <error_dir>", args[0]);
        std::process::exit(1);
    }

    let input_dir = Path::new(&args[1]);
    let output_dir = Path::new(&args[2]);
    let error_dir = Path::new(&args[3]);

    println!("Input:  {}", input_dir.display());
    println!("Output: {}", output_dir.display());
    println!("Error:  {}", error_dir.display());

    let files = collect_files(input_dir)?;

    if files.is_empty() {
        println!("No files found in input directory.");
        return Ok(());
    }

    println!("Files found ({}):", files.len());
    for f in &files {
        println!("  {}", f);
    }

    let mut session = sap_login()?;

    for file_path in &files {
        println!("\nProcessing: {}", file_path);

        let input_path = Path::new(file_path);

        // --- Read and parse the JSON file ---
        let file_content = match fs::read_to_string(input_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Failed to read {}: {}", file_path, e);
                let _ = fs::copy(input_path, error_dir.join(input_path.file_name().unwrap()));
                continue;
            }
        };

        // --- Deserialize into Orders struct ---
        // FIX 1: Orders is a typed struct — index with [] was wrong,
        //         access fields directly via .field_name
        let mut order: Orders = match serde_json::from_str(&file_content) {
            Ok(o)  => o,
            Err(e) => {
                eprintln!("Failed to parse JSON in {}: {}", file_path, e);
                let _ = fs::copy(input_path, error_dir.join(input_path.file_name().unwrap()));
                continue;
            }
        };

        // FIX 2: Access ShipToCode via struct field, not JSON indexing
        //         ship_to_code is Option<String> so clone the inner value
        let gln = match order.ship_to_code.clone() {
            Some(code) => code,
            None => {
                eprintln!("❌ Missing ShipToCode in {}", file_path);
                let _ = fs::copy(input_path, error_dir.join(input_path.file_name().unwrap()));
                continue;
            }
        };

        // FIX 3: get_bp_address now takes card_code, gln, card_type as separate args
        //         (removed the comma-separated string hack)
        let ship_to_name = match get_bp_address(&mut session, &order.card_code, &gln, "cCustomer") {
            Ok(Some(name)) => name,
            Ok(None) => {
                eprintln!("⚠️  No ShipTo found for CardCode={} GLN={}", order.card_code, gln);
                let _ = fs::copy(input_path, error_dir.join(input_path.file_name().unwrap()));
                continue;
            }
            Err(e) => {
                eprintln!("❌ BP address lookup failed for {}: {}", file_path, e);
                let _ = fs::copy(input_path, error_dir.join(input_path.file_name().unwrap()));
                continue;
            }
        };

        // FIX 4: Update ShipToCode on the struct field directly
        println!("✅ ShipTo resolved: {}", ship_to_name);
        order.ship_to_code = Some(ship_to_name);

        // FIX 5: create_order and output writing were outside the for loop
        //         due to mismatched braces — moved inside correctly
        match create_order(&mut session, &order) {
            Ok(created) => {
                // Serialize struct back to Value, then inject SAP response fields
                let mut output = serde_json::to_value(&order)?;
                output["sap_doc_num"]   = Value::Number(created.doc_num.into());
                output["sap_doc_entry"] = Value::Number(created.doc_entry.into());

                let file_name   = input_path.file_name().ok_or("Invalid file name")?;
                let output_path = output_dir.join(file_name);

                fs::write(&output_path, serde_json::to_string_pretty(&output)?)?;
                println!("✅ Written to: {}", output_path.display());
            }
            Err(e) => {
                eprintln!("❌ Order creation failed for {}: {}", file_path, e);
                let _ = fs::copy(input_path, error_dir.join(input_path.file_name().unwrap()));
            }
        }
    }   // ← end of for loop

    println!("\n✅ All files processed.");
    Ok(())
}