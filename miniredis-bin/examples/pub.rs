use mini_redis::{Client, Result};
use miniredis_bin::SERVER_ADDR;

#[tokio::main]
async fn main() -> Result<()> {
    let mut client = Client::connect(SERVER_ADDR).await?;
    // 发布一些数据
    client.publish("numbers", "1".into()).await?;
    client.publish("numbers", "two".into()).await?;
    client.publish("numbers", "3".into()).await?;
    client.publish("numbers", "four".into()).await?;
    client.publish("numbers", "five".into()).await?;
    client.publish("numbers", "6".into()).await?;

    client.publish("foo", "bar".into()).await?;
    Ok(())
}
