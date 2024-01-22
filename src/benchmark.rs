extern crate rdkafka;
use std::time::Instant;

use chrono::Utc;
use rand::Rng;
use rdkafka::{
    producer::{BaseProducer, BaseRecord},
    ClientConfig,
};
use serde_derive::Serialize;

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

fn generate_rand_order() -> BookOrder {
    let mut rng = rand::thread_rng();

    // Calculate new range for price
    let min_price = 41_933_700_000_000_000 / 100_000_000_000;
    let max_price = 43_537_800_000_000_000 / 100_000_000_000;
    let price_step = (max_price - min_price) / 99;
    let price = (min_price + price_step * rng.gen_range(0..=99)) * 100_000_000_000;

    // Calculate new range for quantity
    let min_quantity = 187_000_000_000 / 1_000_000_000;
    let max_quantity = 617_000_000_000 / 1_000_000_000;
    let quantity_step = (max_quantity - min_quantity) / 99;
    let quantity = (min_quantity + quantity_step * rng.gen_range(0..=99)) * 1_000_000_000;

    BookOrder {
        is_buy: rng.gen_bool(0.5),
        reduce_only: rng.gen_bool(0.5),
        quantity,
        price,
        timestamp: Utc::now().timestamp(),
        trigger_price: rng.gen_range(1..=10_000),
        leverage: rng.gen_range(1..=10),
        expiration: rng.gen_range(1..=10_000),
        hash: rng.gen::<u128>().to_string(),
        salt: rng.gen::<u128>(),
        maker: rng.gen::<u128>().to_string(),
        flags: rng.gen::<u128>().to_string(),
    }
}

fn generate_rand_orders(size: usize) -> Vec<BookOrder> {
    let mut orders = Vec::with_capacity(size);

    for _ in 0..size {
        let order = generate_rand_order();
        orders.push(order);
    }

    orders
}

fn main() {
    // Create Kafka producer configuration
    let producer: BaseProducer = ClientConfig::new()
        .set("bootstrap.servers", "localhost:9092")
        .create()
        .expect("Producer creation error");

    println!("producer connected");

    let num_of_orders = 100000;

    let random_orders = generate_rand_orders(num_of_orders);

    let start_time = Instant::now();

    for order in random_orders {
        let json_string = serde_json::to_string(&order).expect("JSON serialization error");

        let delivery = producer.send(BaseRecord::to(&"test").key("").payload(&json_string));

        match delivery {
            Ok(_delivery) => continue,
            Err((e, _)) => println!("Error: {:?}", e),
        }
    }

    let end_time = Instant::now();
    let duration = end_time - start_time;

    println!(
        "duration to publish {} orders to kafka: {:?}",
        num_of_orders, duration
    );
}
