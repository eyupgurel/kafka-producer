use std::time::Duration;

use rdkafka::producer::{FutureProducer, FutureRecord, Producer};
use rdkafka::ClientConfig;
use serde_json::json;

#[tokio::main]
async fn main() {
    // Create Kafka producer configuration
    let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", "localhost:9092")
        .create()
        .expect("Producer creation error");

    let json_value = json!({
        "name": "John Doe",
        "age": 30,
        "city": "New York",
    });

    let json_string = serde_json::to_string(&json_value).expect("JSON serialization error");

    let delivery = producer.send(
        FutureRecord::to(&"test")
            .key("")
            .payload(&json_string),
        Duration::from_secs(0),
    );

    println!("producer connected");

    match delivery.await {
        Ok(delivery) => println!("Sent: {:?}", delivery),
        Err((e, _)) => println!("Error: {:?}", e),
    }

    // // Poll at regular intervals to process all the asynchronous delivery events.
    for _ in 0..10 {
        producer.poll(Duration::from_millis(100));
    }

    // // And/or flush the producer before dropping it.
    producer
        .flush(Duration::from_secs(1))
        .expect("Flush failed");
}
