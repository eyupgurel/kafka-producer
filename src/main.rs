mod models;
use crate::models::common::{generate_random_string, BinanceOrderBook, Config};
use chrono::Utc;
use rand::Rng;
use std::sync::mpsc;
use std::time::Duration;
use std::{fs, string};

use rdkafka::producer::{FutureProducer, FutureRecord, Producer};
use rdkafka::ClientConfig;
use serde_json::json;
use serde::Serialize;
use std::time::Instant;

#[derive(Serialize)]
pub struct BookOrder {
    pub is_buy: bool,

    pub reduce_only: bool,
    // quantity of asset to be traded
    pub quantity: u128,
    // price at which trade is to be made
    pub price: u128,
    // time of the order
    pub timestamp: i64,
    // stop order price
    pub trigger_price: u128,
    // leverage (in whole numbers) for trade
    pub leverage: u128,
    // time after which order becomes invalid
    pub expiration: u128,
    // order hash
    pub hash: String,
    // random number,
    pub salt: u128,
    // address of order maker
    pub maker: String,
    // /// encoded order flags, isBuy, decreasOnly
    pub flags: String,
}

#[tokio::main]
async fn main() {
    // Create Kafka producer configuration
    let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", "localhost:9092")
        .create()
        .expect("Producer creation error");

    println!("producer connected");

    let config_str =
        fs::read_to_string("src/config/config.json").expect("Unable to read config.json");
    let config: Config = serde_json::from_str(&config_str).expect("JSON was not well-formatted");
    let market = config.markets.first().unwrap();
    let binance_market = market.symbols.binance.to_owned();
    let binance_market_for_ob = binance_market.clone();
    let binance_market_for_depth_diff = binance_market.clone();

    //let (tx_binance_ob, rx_binance_ob) = mpsc::channel();
    let (tx_binance_depth_diff, rx_binance_depth_diff) = mpsc::channel::<BinanceOrderBook>();

    let client = reqwest::blocking::Client::new();

    let order_book_result: Result<BinanceOrderBook, Box<dyn std::error::Error>> = client
        .get(&"https://fapi.binance.com/fapi/v1/depth?symbol=BTCUSDT&limit=1000".to_string())
        .send()
        .map_err(|e| format!("Error making the request: {}", e).into())
        .and_then(|res| {
            res.text()
                .map_err(|e| format!("Error reading the response body: {}", e).into())
        })
        .and_then(|body| serde_json::from_str(&body).map_err(Into::into));

    let order_book = order_book_result
        .map(|response| response)
        .map_err(|e| {
            tracing::error!("Error: {}", e);
            e
        })
        .unwrap();

    let start = Instant::now();

    for bid in order_book.bids.iter() {
        // Convert the bid to a BookOrder
        let book_order = BookOrder {
            is_buy: true,       // Assuming these are buy orders; adjust if necessary
            reduce_only: false, // Set according to your logic
            quantity: bid.1 as u128,
            price: bid.0 as u128,
            timestamp: Utc::now().timestamp(),
            trigger_price: 0,                 // Set according to your logic
            leverage: 1,                      // Set according to your logic
            expiration: 0,                    // Set according to your logic
            hash: generate_random_string(10), // Generate a unique hash for the order
            salt: rand::random(),
            maker: "SomeMaker".to_string(), // Replace with actual maker
            flags: "SomeFlags".to_string(), // Replace with actual flags
        };

        let json_string = serde_json::to_string(&book_order).expect("JSON serialization error");

        let delivery = producer.send(
            FutureRecord::to(&"test").key("").payload(&json_string),
            Duration::from_secs(0),
        );

        match delivery.await {
            Ok(_delivery) => continue,
            Err((e, _)) => println!("Error: {:?}", e),
        }
    }

    let duration =  Instant::now() - start;
    println!("Duration: {:?}",duration);

    // let json_value = json!({
    //     "name": "John Doe",
    //     "age": 30,
    //     "city": "New York",
    // });

    // let json_string = serde_json::to_string(&json_value).expect("JSON serialization error");

    // let delivery = producer.send(
    //     FutureRecord::to(&"test")
    //         .key("")
    //         .payload(&json_string),
    //     Duration::from_secs(0),
    // );

    // println!("producer connected");

    // match delivery.await {
    //     Ok(delivery) => println!("Sent: {:?}", delivery),
    //     Err((e, _)) => println!("Error: {:?}", e),
    // }

    // // Poll at regular intervals to process all the asynchronous delivery events.
    for _ in 0..10 {
        producer.poll(Duration::from_millis(100));
    }

    // // And/or flush the producer before dropping it.
    producer
        .flush(Duration::from_secs(1))
        .expect("Flush failed");
}
