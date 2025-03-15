#[tokio::main]
async fn main() {
    let session = zenoh::open(zenoh::Config::default()).await.unwrap();
    let subscriber = session.declare_subscriber("topic").await.unwrap();
    while let Ok(sample) = subscriber.recv() {
        let s = sample.payload().try_to_string().unwrap().into_owned();
        println!("Received: {}", s);
    }
}
