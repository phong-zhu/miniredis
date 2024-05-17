use mini_redis::Client;
use mini_redis::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let mut client = Client::connect("127.0.0.1:6379").await.unwrap();
    let res = client.get("hello").await?;
    println!("{:?}", res);
    Ok(())
}