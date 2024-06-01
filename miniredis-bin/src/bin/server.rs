use bytes::Bytes;
use mini_redis::run;
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
    run(listener, signal::ctrl_c()).await;
    Ok(())
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
