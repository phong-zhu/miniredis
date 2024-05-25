
use mini_redis::{Client, Result};
use miniredis_bin::SERVER_ADDR;

#[tokio::main]
async fn main() {
    let mut client = Client::connect(SERVER_ADDR);
    println!("hello");
}