use bytes::Bytes;
use mini_redis::Client;
use mini_redis::Result;

//noinspection ALL
#[tokio::main]
async fn main() -> Result<()> {
    let mut client = Client::connect("127.0.0.1:6379").await.unwrap();
    let ping_res = client.ping(Some(Bytes::from("ping"))).await?;
    println!("ping {:?}", ping_res);
    Ok(())
}