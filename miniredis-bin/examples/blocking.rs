use mini_redis::{BlockingClient, Result};

fn main() -> Result<()> {
    let mut client = match BlockingClient::connect("localhost:6379") {
        Ok(client) => client,
        Err(_) => panic!("failed to establish connection"),
    };
    client.set("hello", "world".into())?;
    let result = client.get("hello")?;
    println!("got value from the server; success={:?}", result.is_some());
    // drop(client);
    Ok(())
}
