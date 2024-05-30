use mini_redis::{Client, Result};
use miniredis_bin::SERVER_ADDR;

#[tokio::main]
async fn main() -> Result<()> {
    let mut client = Client::connect(SERVER_ADDR).await?;
    let mut subscriber = client.subscribe(vec!["numbers".to_string()]).await?;
    while let Some(msg) = subscriber.next_message().await? {
        println!("subscribe got {}, {:?}", msg.channel, msg.content);
    }
    Ok(())
}