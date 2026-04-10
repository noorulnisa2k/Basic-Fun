// mod order_structure_test;

// use dotenv::dotenv;
// use std::env;

// // use std::fmt::format;
// use log::{info, warn, error, debug};
// use std::path::Path;
// use std::fs;
// use order_structure_test::Orders;

// fn main() {

//     dotenv().ok(); // 👈 load .env file

//     let api_url = env::var("API_URL").expect("API_URL not set");
//     let api_key = env::var("API_KEY").expect("API_KEY not set");

//     println!("URL: {}", api_url);
//     println!("Key: {}", api_key);

//     println!("---------------");

//     let args: Vec<String> = env::args().collect();
//     println!("{:?}, lenght: {}", args, args.len());

//     let input_dir = &args[1];
//     let output_dir = &args[2];
//     println!("{}, {}", input_dir, output_dir);


//     for file in fs::read_dir(input_dir).expect("Failed to read input dir"){
//         let file = file.expect("Failed to get the files");
//         let file_path = file.path();

//         if file_path.is_dir(){
//             continue;
//         }
        
//         let data = fs::read_to_string(&file_path)
//             .expect("Failed to read file");

//         let order: Orders = serde_json::from_str(&data)
//             .expect("Invalid JSON");

//         let new_name = format!("{}.json", order.card_code);

//         let output_path = Path::new(output_dir).join(new_name);

//         let output_json = serde_json::to_string_pretty(&order).unwrap();

//         fs::write(output_path, output_json)
//         .expect("Failed to write the file");

//     }

//     env_logger::init();

//     info!("this is a infor msg");
//     warn!("this is a warn msg");
//     debug!("this is a debug msg");
//     error!("this is a error message");
    

    
//     // println!("{:#?}", order);
// }


// use reqwest::blocking::Client;
// use serde_json::json;

// fn main() {
//     let client = Client::new();

//     let res = client
//         .post("https://fakestoreapi.com/users")
//         .json(&json!({
//             "title": "hello",
//             "body": "world",
//             "userId": 1
//         }))
//         .send()
//         .unwrap();

//     println!("{}", res.text().unwrap());

//     }


// -----------------------------------------

// use reqwest::Client;
// use serde::{Deserialize, Serialize};

// const BASE_URL: &str = "http://127.0.0.1:8000";
// const TOKEN: &str = "mysecrettoken";

// #[derive(Serialize, Deserialize, Debug)]
// struct Order {
//     id: i32,
//     item: String,
//     quantity: i32,
//     price: f64,
// }

// // -----------------------------
// // POST - Create Order
// // -----------------------------
// async fn create_order(client: &Client) -> Result<(), reqwest::Error> {
//     let order = Order {
//         id: 3,
//         item: "Laptop".to_string(),
//         quantity: 2,
//         price: 1200.0,
//     };

//     let res = client
//         .post(&format!("{}/orders", BASE_URL))
//         .bearer_auth(TOKEN)
//         .json(&order)
//         .send()
//         .await?;

//     let body = res.text().await?;
//     println!("POST Response: {}", body);

//     Ok(())
// }

// // -----------------------------
// // GET - Fetch Order
// // -----------------------------
// async fn get_order(client: &Client) -> Result<(), reqwest::Error> {
//     let res = client
//         .get(&format!("{}/orders/1", BASE_URL))
//         .bearer_auth(TOKEN)
//         .send()
//         .await?;

//     let body = res.text().await?;
//     println!("GET Response: {}", body);

//     Ok(())
// }

// // -----------------------------
// // PUT - Update Order
// // -----------------------------
// async fn update_order(client: &Client) -> Result<(), reqwest::Error> {
//     let updated_order = Order {
//         id: 1,
//         item: "Updated Laptop".to_string(),
//         quantity: 5,
//         price: 1500.0,
//     };

//     let res = client
//         .put(&format!("{}/orders/1", BASE_URL))
//         .bearer_auth(TOKEN)
//         .json(&updated_order)
//         .send()
//         .await?;

//     let body = res.text().await?;
//     println!("PUT Response: {}", body);

//     Ok(())
// }

// // -----------------------------
// // MAIN
// // -----------------------------
// #[tokio::main]
// async fn main() -> Result<(), Box<dyn std::error::Error>> {
//     let client = Client::new();

//     // 1. Create
//     create_order(&client).await?;

//     // 2. Get
//     get_order(&client).await?;

//     // 3. Update
//     update_order(&client).await?;

//     // 4. Get again
//     get_order(&client).await?;

//     Ok(())
// }

// -----------------------------------
use reqwest::Client;
use serde::{Deserialize, Serialize};
use futures::stream::{self, StreamExt};

const BASE_URL: &str = "http://127.0.0.1:8000";
const TOKEN: &str = "mysecrettoken";

#[derive(Serialize, Deserialize, Debug)]
struct Order {
    id: i32,
    item: String,
    quantity: i32,
    price: f64,
}

// -----------------------------
// GET single order
// -----------------------------
async fn get_order(client: &Client, id: i32) {
    let res = client
        .get(&format!("{}/orders/{}", BASE_URL, id))
        .bearer_auth(TOKEN)
        .send()
        .await;

    match res {
        Ok(resp) => {
            let text = resp.text().await.unwrap_or("Failed to read".to_string());
            println!("GET {} → {}", id, text);
        }
        Err(e) => {
            println!("ERROR {} → {}", id, e);
        }
    }
}

// -----------------------------
// POST single order
// -----------------------------
async fn create_order(client: &Client, order: Order) {
    let res = client
        .post(&format!("{}/orders", BASE_URL))
        .bearer_auth(TOKEN)
        .json(&order)
        .send()
        .await;

    match res {
        Ok(resp) => {
            let text = resp.text().await.unwrap_or("Failed to read".to_string());
            println!("POST {} → {}", order.id, text);
        }
        Err(e) => {
            println!("ERROR POST {} → {}", order.id, e);
        }
    }
}

// -----------------------------
// MAIN (parallel with limit)
// -----------------------------
#[tokio::main]
async fn main() {
    let client = Client::new();

    // -----------------------------
    // 1. CREATE multiple orders
    // -----------------------------
    let orders = vec![
        Order { id: 1, item: "Laptop".into(), quantity: 1, price: 1000.0 },
        Order { id: 2, item: "Phone".into(), quantity: 2, price: 500.0 },
        Order { id: 3, item: "Tablet".into(), quantity: 3, price: 300.0 },
        Order { id: 4, item: "Mouse".into(), quantity: 5, price: 50.0 },
    ];

    stream::iter(orders)
        .for_each_concurrent(2, |order| {
            let client = &client;
            async move {
                create_order(client, order).await;
            }
        })
        .await;

    println!("--- All orders created ---");

    // -----------------------------
    // 2. FETCH multiple orders
    // -----------------------------
    let ids = vec![1, 2, 3, 4];

    stream::iter(ids)
        .for_each_concurrent(2, |id| {
            let client = &client;
            async move {
                get_order(client, id).await;
            }
        })
        .await;

    println!("--- All orders fetched ---");
}