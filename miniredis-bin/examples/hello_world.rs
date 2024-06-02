use bytes::Bytes;
use mini_redis::Client;
use mini_redis::Result;
use miniredis_bin::SERVER_ADDR;

#[tokio::main]
async fn main() -> Result<()> {
    let mut client = Client::connect(SERVER_ADDR).await.unwrap();
    let mut get_res = client.get("hello").await?;
    println!("get {:?}", get_res);
    client.set("hello", "world".into()).await?;
    get_res = client.get("hello").await?;
    print!("get {:?}", get_res);
    Ok(())
}
