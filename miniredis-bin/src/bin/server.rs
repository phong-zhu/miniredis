
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use bytes::Bytes;
use mini_redis::{Connection, Frame, Result};
use tokio::net::{TcpListener, TcpStream};
use miniredis_bin::SERVER_ADDR;
use mini_redis::Db;


#[tokio::main]
async fn main() -> Result<()>{
    let listener = TcpListener::bind(SERVER_ADDR).await.unwrap();
    println!("listening");
    let db = Db::new();
    loop {
        let (socket, _) = listener.accept().await?;
        let mut connection = Connection::new(socket);
        let db = db.clone();
        tokio::spawn(async move {
            process(&mut connection, &db).await;
        });
    }
}

async fn process(connection: &mut Connection, db: &Db) -> crate::Result<()> {
    use mini_redis::Command::{self, Get, Set, Ping, Subscribe};
    while let Some(frame) = connection.read_frame().await.unwrap() {
        match Command::from_frame(frame).unwrap() {
            Get(cmd) => {
                cmd.apply(db, connection).await?;
            }
            Set(cmd) => {
                cmd.apply(db, connection).await?;
            }
            Ping(cmd) => {
                cmd.apply(db, connection).await?;
            }
            Subscribe(cmd) => {
                cmd.apply(db, connection).await?;
            }
            cmd => {
                let error = Frame::Error("unimplemented".to_string());
                connection.write_frame(&error).await?;
            }
        };
    }
    Ok(())
    // println!("{}", mini_redis::add(1,1));
}
