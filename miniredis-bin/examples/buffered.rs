use bytes::Bytes;
use mini_redis::Result;
use mini_redis::{BufferedClient, Client};
use miniredis_bin::SERVER_ADDR;

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::connect(SERVER_ADDR).await?;
    let mut buffered = BufferedClient::buffer(client);

    buffered.set("hello", "world".into()).await?;
    let result = buffered.get("hello").await?;
    println!("got value from the server; success={:?}", result.is_some());

    Ok(())
}
