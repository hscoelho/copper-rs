#[tokio::main]
async fn main() {
    let session = zenoh::open(zenoh::Config::default()).await.unwrap();
    session.put("topic", "value").await.unwrap();
    session.close().await.unwrap();
}
