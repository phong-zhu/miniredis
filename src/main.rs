
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use bytes::Bytes;
use mini_redis::{Connection, Frame};
use tokio::net::{TcpListener, TcpStream};

type Db = Arc<Mutex<HashMap<String, Bytes>>>;

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:6379").await.unwrap();
    println!("listening");
    let db = Arc::new(Mutex::new(HashMap::new()));
    loop {
        let (socket, _) = listener.accept().await.unwrap();
        let db = db.clone();
        tokio::spawn(async move {
            process(socket, db).await;
        });
    }
}

async fn process(socket: TcpStream, db: Db) {
    use mini_redis::Command::{self, Get, Set};

    let mut connection = Connection::new(socket);
    while let Some(frame) = connection.read_frame().await.unwrap() {
        let response: Frame = match Command::from_frame(frame).unwrap() {
            Set(cmd) => {
                Frame::Error("unimplemented".to_string())
            }
            Get(cmd) => {
                Frame::Error("unimplemented".to_string())
            }
            cmd => {
                Frame::Error("unimplemented".to_string())
            }
        };
        connection.write_frame(&response).await.unwrap();
    }
    // println!("{}", mini_redis::add(1,1));
}
