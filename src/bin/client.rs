use bytes::Bytes;
use mini_redis::Client;
use mini_redis::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let mut client = Client::connect("127.0.0.1:6379").await.unwrap();
    let get_res = client.get("hello").await?;
    println!("get {:?}", get_res);
    let ping_res = client.ping(Some(Bytes::from("ping"))).await?;
    println!("ping {:?}", ping_res);
    Ok(())
}