
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use bytes::Bytes;
use mini_redis::{Connection, Frame, Result};
use tokio::net::{TcpListener, TcpStream};
use miniredis_bin::SERVER_ADDR;

type Db = Arc<Mutex<HashMap<String, Bytes>>>;

#[tokio::main]
async fn main() -> Result<()>{
    let listener = TcpListener::bind(SERVER_ADDR).await.unwrap();
    println!("listening");
    let db = Arc::new(Mutex::new(HashMap::new()));
    loop {
        let (socket, _) = listener.accept().await.unwrap();
        let db = db.clone();
        tokio::spawn(async move {
            process(socket, &db).await;
        });
    }
}

async fn process(socket: TcpStream, db: &Db) {
    use mini_redis::Command::{self, Get, Set, Ping, Subscribe};

    let mut connection = Connection::new(socket);
    while let Some(frame) = connection.read_frame().await.unwrap() {
        let response = match Command::from_frame(frame).unwrap() {
            Set(cmd) => {
                let mut db = db.lock().unwrap();
                db.insert(cmd.key().to_string(), cmd.value().clone());
                Frame::Simple("OK".to_string())
            }
            Get(cmd) => {
                let db = db.lock().unwrap();
                if let Some(value) = db.get(cmd.key()) {
                    Frame::Bulk(value.clone())
                } else {
                    Frame::Null
                }
            }
            Ping(cmd) => {
                Frame::Bulk(cmd.get_msg().unwrap())
            }
            // Subscribe(cmd) => cmd.apply(db, dst, shutdown).await,
            cmd => {
                Frame::Error("unimplemented".to_string())
            }
        };
        connection.write_frame(&response).await.unwrap();
    }
    // println!("{}", mini_redis::add(1,1));
}
