use mini_redis::{Client, Result};
use miniredis_bin::SERVER_ADDR;
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() -> Result<()> {
    let mut client = Client::connect(SERVER_ADDR).await?;
    let mut subscriber = client.subscribe(vec!["numbers".to_string()]).await?;
    let messages = subscriber.into_stream();
    tokio::pin!(messages);
    while let Some(v) = messages.next().await {
        println!("GOT = {:?}", v);
    }
    Ok(())
}
