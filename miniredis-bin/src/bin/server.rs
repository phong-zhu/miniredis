use bytes::Bytes;
use mini_redis::Db;
use mini_redis::Shutdown;
use mini_redis::{Connection, Frame, Result};
use miniredis_bin::SERVER_ADDR;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::{thread, time};
use tokio::net::{TcpListener, TcpStream};
use tokio::select;
use tokio::signal;
use tokio::sync::broadcast;
use tracing::{debug, error, info, instrument};

#[tokio::main]
async fn main() -> Result<()> {
    let listener = TcpListener::bind(SERVER_ADDR).await.unwrap();
    println!("listening");
    let db = Db::new();
    let (notify_shutdown, _) = broadcast::channel(1);

    loop {
        let (socket, _) = listener.accept().await?;
        let mut connection = Connection::new(socket);
        let db = db.clone();
        let notify_shutdown = notify_shutdown.clone();
        let mut shutdown = Shutdown::new(notify_shutdown.subscribe());
        tokio::spawn(async move {
            select! {
                res = process(&mut connection, &db, &mut shutdown) => {
                    if let Err(err) = res {
                        error!(cause=%err, "fail to process")
                    }
                }
                // _ = signal::ctrl_c() => {
                //     println!("shutting down by ctrl c");
                // }
            }
        });
    }
}

async fn process(
    connection: &mut Connection,
    db: &Db,
    shutdown: &mut Shutdown,
) -> crate::Result<()> {
    use mini_redis::Command::{self, Get, Ping, Publish, Set, Subscribe};
    while !shutdown.is_shutdown() {
        let maybe_frame = tokio::select! {
            res = connection.read_frame() => res?,
            _ = shutdown.recv() => None,
        };

        let frame = match maybe_frame {
            Some(frame) => frame,
            None => return Ok(()),
        };

        let command = Command::from_frame(frame)?;
        debug!(?command);
        command.apply(db, connection, shutdown).await?;
    }
    Ok(())
}
