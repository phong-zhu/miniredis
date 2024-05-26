
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
            process(connection, &db).await;
        });
    }
}

async fn process(mut connection: Connection, db: &Db) {
    use mini_redis::Command::{self, Get, Set, Ping, Subscribe};
    while let Some(frame) = connection.read_frame().await.unwrap() {
        let response = match Command::from_frame(frame).unwrap() {
            Set(cmd) => {
                db.set(cmd.key().to_string(), cmd.value().clone(), cmd.expire());
                Frame::Simple("OK".to_string())
            }
            Get(cmd) => {
                if let Some(value) = db.get(cmd.key()) {
                    Frame::Bulk(value.clone())
                } else {
                    Frame::Null
                }
            }
            Ping(cmd) => {
                Frame::Bulk(cmd.get_msg().unwrap())
            }
            Subscribe(cmd) => {
                let frame = cmd.apply(db, &mut connection).await;
                match frame {
                    Ok(frame) => frame,
                    _ => Frame::Error("subscribe err".to_string()),
                }
            }
            cmd => {
                Frame::Error("unimplemented".to_string())
            }
        };
        connection.write_frame(&response).await.unwrap();
    }
    // println!("{}", mini_redis::add(1,1));
}
