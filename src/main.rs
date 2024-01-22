mod models;
mod sockets;
mod benchmark;
use crate::models::common::DepthUpdate;
use crate::models::common::{generate_random_string, BinanceOrderBook, Config};
use crate::sockets::binance_depth_update_socket::BinanceDepthUpdateStream;
use crate::sockets::common::DepthUpdateStream;
use chrono::Utc;
use std::sync::mpsc;
use std::{fs, thread};

use rdkafka::producer::{BaseProducer, BaseRecord};
use rdkafka::ClientConfig;
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

fn main() {
    // Create Kafka producer configuration
    let producer:BaseProducer = ClientConfig::new()
        .set("bootstrap.servers", "localhost:9092")
        .create()
        .expect("Producer creation error");

    println!("producer connected");

    let config_str =
        fs::read_to_string("src/config/config.json").expect("Unable to read config.json");
    let config: Config = serde_json::from_str(&config_str).expect("JSON was not well-formatted");
    let market = config.markets.first().unwrap();
    let binance_market = market.symbols.binance.to_owned();
    let binance_market_for_depth_diff = binance_market.clone();

    //let (tx_binance_ob, rx_binance_ob) = mpsc::channel();
    let (tx_binance_depth_diff, rx_binance_depth_diff) = mpsc::channel::<DepthUpdate>();

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
            hash: generate_random_string(40), // Generate a unique hash for the order
            salt: rand::random(),
            maker: generate_random_string(10), // Assuming 'maker' is defined in your scope
            flags: generate_random_string(10), // Assuming 'flags' are defined in your scope
        };

        let json_string = serde_json::to_string(&book_order).expect("JSON serialization error");

        let delivery = producer.send(BaseRecord::to(&"test").key("").payload(&json_string));

        match delivery {
            Ok(_delivery) => continue,
            Err((e, _)) => println!("Error: {:?}", e),
        }
    }

    let duration = Instant::now() - start;
    println!("Duration to publish orders from api in kafka: {:?}", duration);

    let binance_websocket_url_for_depth_diff = "wss://fstream.binance.com";

    // Now you can use binance_websocket_url_for_depth_diff for the second thread
    thread::spawn(move || {
        let diff_depth_stream =
            BinanceDepthUpdateStream::<crate::models::common::DepthUpdate>::new();
        let url = format!(
            "{}/stream?streams={}@depth@100ms",
            &binance_websocket_url_for_depth_diff, &binance_market_for_depth_diff
        );
        diff_depth_stream.stream_depth_update_socket(
            &url,
            &binance_market_for_depth_diff,
            tx_binance_depth_diff,
        );
    });

    loop {
        match rx_binance_depth_diff.try_recv() {
            Ok(value) => {
                tracing::info!("bids count: {:?}", value.data.bids.len());

                let start_time = Instant::now();

                for bid in &value.data.bids {
                    // Convert the bid to a BookOrder
                    let book_order = BookOrder {
                        is_buy: true,       // Assuming these are buy orders; adjust if necessary
                        reduce_only: false, // Set according to your logic
                        quantity: bid.1 as u128,
                        price: bid.0 as u128,
                        timestamp: Utc::now().timestamp(),
                        trigger_price: 0,        // Set according to your logic
                        leverage: 1,             // Set according to your logic
                        expiration: 0,           // Set according to your logic
                        hash: generate_random_string(40), // Generate a unique hash for the order
                        salt: rand::random(),
                        maker: generate_random_string(10), // Assuming 'maker' is defined in your scope
                        flags: generate_random_string(10), // Assuming 'flags' are defined in your scope
                    };
                    // if(bid.1 > 0) {
                    //     // Upsert the order in the in-memory order book
                    //     upsert_order(&mut native_order_book, book_order);

                    // } else if (bid.1 == 0) {
                    //     delete_order(&mut native_order_book, book_order.price, book_order.timestamp, &book_order.hash);
                    // }

                    let json_string =
                        serde_json::to_string(&book_order).expect("JSON serialization error");

                    let delivery =
                        producer.send(BaseRecord::to(&"test").key("").payload(&json_string));

                    match delivery {
                        Ok(_delivery) => println!("Order Published to kafka from stream: {:?}",json_string),
                        Err((e, _)) => println!("Error: {:?}", e),
                    }
                }

                let end_time = Instant::now();
                let duration = end_time - start_time;
                tracing::info!("orderbook update duration: {:?}", duration);

                tracing::info!("binance depth diff: {:?}", value);
            }
            Err(mpsc::TryRecvError::Empty) => {
                // No message from binance yet
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                tracing::debug!("Binance worker has disconnected!");
            }
        }
    }

    // // // Poll at regular intervals to process all the asynchronous delivery events.
    // for _ in 0..10 {
    //     producer.poll(Duration::from_millis(100));
    // }

    // // // And/or flush the producer before dropping it.
    // producer
    //     .flush(Duration::from_secs(1))
    //     .expect("Flush failed");
}
