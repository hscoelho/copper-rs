#[tokio::main]
async fn main() {
    let session = zenoh::open(zenoh::Config::default()).await.unwrap();
    let subscriber = session.declare_subscriber("topic").await.unwrap();
    while let Ok(sample) = subscriber.recv_async().await {
        println!("Received: {:?}", sample);
    }
}
