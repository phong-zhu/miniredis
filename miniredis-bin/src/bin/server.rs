
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::{thread, time};
use mini_redis::Shutdown;
use bytes::Bytes;
use mini_redis::{Connection, Frame, Result};
use tokio::net::{TcpListener, TcpStream};
use tokio::select;
use miniredis_bin::SERVER_ADDR;
use mini_redis::Db;
use tracing::{debug, error, info, instrument};
use tokio::signal;
use tokio::sync::broadcast;

#[tokio::main]
async fn main() -> Result<()>{
    let listener = TcpListener::bind(SERVER_ADDR).await.unwrap();
    println!("listening");
    let db = Db::new();
    let (notify_shutdown, _) = broadcast::channel(1);

    loop {
        let (socket, _) = listener.accept().await?;
        let mut connection = Connection::new(socket);
        let db = db.clone();
        let notify_shutdown = notify_shutdown.clone();
        let shutdown = Shutdown::new(notify_shutdown.subscribe());
        tokio::spawn(async move {
            select! {
                res = process(&mut connection, &db, shutdown) => {
                    if let Err(err) = res {
                        error!(cause=%err, "fail to process")
                    }
                }
                _ = signal::ctrl_c() => {
                    println!("shutting down by ctrl c");
                }
            }

        });
    }
}

async fn process(connection: &mut Connection, db: &Db, shutdown: Shutdown) -> crate::Result<()> {
    use mini_redis::Command::{self, Get, Set, Ping, Subscribe, Publish};
    while !shutdown.is_shutdown() {
        if let Some(frame) = connection.read_frame().await.unwrap() {
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
                Publish(cmd) => {
                    cmd.apply(db, connection).await?;
                }
                cmd => {
                    let error = Frame::Error("unimplemented".to_string());
                    connection.write_frame(&error).await?;
                }
            };
        }
    }
    Ok(())
    // println!("{}", mini_redis::add(1,1));
}
