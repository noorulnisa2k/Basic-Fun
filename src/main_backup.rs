use std::env;
use std::fs;
use std::path::Path;

use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;

// const BASE_URL: &str = "https://your_server:50000/b1s/v1";


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

#[derive(Debug)]
struct SapSession {
    session_id: String,
    cookies: String,
}

// fn load_env() {
//     let exe_path = env::current_exe().expect("Cannot find exe path");
//     let exe_dir = exe_path.parent().expect("Cannot find exe directory");

//     let env_path = exe_dir.join(".env");

//     dotenv::from_path(env_path).ok();
// }

fn sap_login() -> Result<SapSession, Box<dyn Error>> {
    dotenv::dotenv().ok();
    // load_env();

    let base_url = env::var("BASE_URL").expect("BASE_URL must be set");
    let company = env::var("Company_DB").expect("CompanyDB must be set");
    let username = env::var("User_Name").expect("UserName must be set");
    let password = env::var("Password").expect("Password must be set");
    println!("{:?}, {:?}, {:?}, {:?}", base_url, company, username, password);

    let client = Client::builder()
        .danger_accept_invalid_certs(true)
        .build()?;

    let url = format!("{}/Login", base_url);

    let payload = LoginRequest {
        company_db: company,
        user_name: username,
        password: password,
    };

    // 1. Print the URL
    println!("--- Debug Info ---");
    println!("URL: {}", url);

    // 2. Serialize the payload to a pretty-printed string to check for hidden errors
    match serde_json::to_string_pretty(&payload) {
        Ok(json) => println!("Body:\n{}", json),
        Err(e) => println!("Failed to serialize payload: {}", e),
    }
    println!("------------------");

    let response = client.post(&url).json(&payload).send()?;

    if !response.status().is_success() {
        // return Err(format!("Login failed: {}", response.status()).into());

        let status = response.status();
        let error_text = response.text().unwrap_or_else(|_| "Could not read error body".to_string());
    
        return Err(format!("Login failed: {}. Details: {}", status, error_text).into());
    }

    // Extract cookies BEFORE consuming response
    let cookies = response
        .headers()
        .get_all("set-cookie")
        .iter()
        .map(|v| v.to_str().unwrap())
        .collect::<Vec<_>>()
        .join("; ");

    // Parse JSON (this consumes response)
    let login_data: LoginResponse = response.json()?;

    let session = SapSession {
        session_id: login_data.session_id,
        cookies,
    };

    Ok(session)
}


fn main() {
    // Get command line arguments
    let args: Vec<String> = env::args().collect();

    // Check if directory argument is provided
   if args.len() < 4 {
        eprintln!("Param Missing");
        std::process::exit(1);
    }
    println!("{:?}, lenght: {}", args, args.len());

    let input_dir = &args[1];
    let output_dir = &args[2];
    let error_dir = &args[3];

    let input_dir_path = Path::new(input_dir);
    let output_dir_path = Path::new(output_dir);
    let error_dir_path = Path::new(error_dir);
    println!("after converting to path: {:?}, {:?}, {:?}", input_dir_path, output_dir_path, error_dir_path);

    // Check if directory exists
    if !input_dir_path.exists() {
        println!("Directory does not exist");
        return;
    }

    if !input_dir_path.is_dir() {
        println!("Provided path is not a directory");
        return;
    }

    // Vector to store file paths
    let mut files: Vec<String> = Vec::new();

    // Read directory entries
    let entries = fs::read_dir(input_dir).expect("Unable to read directory");

    for entry in entries {
        let entry = entry.expect("Error reading entry");
        let file_path = entry.path();

        if file_path.is_file() {
            files.push(file_path.display().to_string());
        }
    }

    match sap_login() {
        Ok(session) => {
            println!("Login Successful");
            println!("Session ID: {}", session.session_id);
            println!("Cookies: {}", session.cookies);
        }
        Err(e) => println!("Error: {}", e),
    }

    // Print stored file paths one by one
    println!("Files found:\n");

    for file in files {
        println!("{}", file);
    }
}


// /Users/noor/Public/Ecom/input_files /Users/noor/Public/Ecom/output_files /Users/noor/Public/Ecom/error_files
// basic_fun.exe C:/Users/BasicFun/Desktop/test/input/ C:/Users/BasicFun/Desktop/test/output/ C:/Users/BasicFun/Desktop/test/error/  