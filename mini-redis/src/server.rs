use crate::shutdown;
use crate::{db::DbDropGuard, Command, Connection, Db, Shutdown};
use std::future::Future;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, mpsc, Semaphore};
use tokio::time::{self, Duration};
use tracing::{debug, error, info, instrument};

const MAX_CONNECTIONS: usize = 250;
const BACKOFF_MAX: u64 = 64;

/// Used as part of the graceful shutdown process to wait for client
/// connections to complete processing.
///
/// Tokio channels are closed once all `Sender` handles go out of scope.
/// When a channel is closed, the receiver receives `None`. This is
/// leveraged to detect all connection handlers completing. When a
/// connection handler is initialized, it is assigned a clone of
/// `shutdown_complete_tx`. When the listener shuts down, it drops the
/// sender held by this `shutdown_complete_tx` field. Once all handler tasks
/// complete, all clones of the `Sender` are also dropped. This results in
/// `shutdown_complete_rx.recv()` completing with `None`. At this point, it
/// is safe to exit the server process.
pub async fn run(listener: TcpListener, shutdown: impl Future) {
    let (notify_shutdown, _) = broadcast::channel(1);
    let (shutdown_complete_tx, mut shutdown_complete_rx) = mpsc::channel(1);
    let mut server = Server::new(
        listener,
        DbDropGuard::new(),
        Arc::new(Semaphore::new(MAX_CONNECTIONS)),
        notify_shutdown,
        shutdown_complete_tx,
    );
    tokio::select! {
        res = server.run() => {
            if let Err(err) = res {
                error!(case = %err, "failed to accept");
            }
        }
        _ = shutdown => {
            info!("shutting down");
        }
    }
    // graceful shutdown
    let Server {
        shutdown_complete_tx,
        notify_shutdown,
        ..
    } = server;
    drop(notify_shutdown);
    drop(shutdown_complete_tx);
    let _ = shutdown_complete_rx.recv().await;
}

#[derive(Debug)]
pub struct Server {
    db_holder: DbDropGuard,
    listener: TcpListener,
    limit_connections: Arc<Semaphore>,
    notify_shutdown: broadcast::Sender<()>,
    shutdown_complete_tx: mpsc::Sender<()>,
}

impl Server {
    fn new(
        listener: TcpListener,
        db_holder: DbDropGuard,
        limit_connections: Arc<Semaphore>,
        notify_shutdown: broadcast::Sender<()>,
        shutdown_complete_tx: mpsc::Sender<()>,
    ) -> Server {
        Server {
            db_holder,
            listener,
            limit_connections,
            notify_shutdown,
            shutdown_complete_tx,
        }
    }

    async fn run(&mut self) -> crate::Result<()> {
        info!("accpting inbound connections");
        loop {
            let permit = self
                .limit_connections
                .clone()
                .acquire_owned()
                .await
                .unwrap();
            let socket = self.accept().await?;
            let mut handler = Handler::new(
                self.db_holder.db(),
                socket,
                Shutdown::new(self.notify_shutdown.subscribe()),
                self.shutdown_complete_tx.clone(),
            );
            tokio::spawn(async move {
                if let Err(err) = handler.run().await {
                    error!(case = ?err, "connection err");
                }
                drop(permit);
            });
        }
    }

    async fn accept(&mut self) -> crate::Result<TcpStream> {
        let mut backoff = 1;
        loop {
            match self.listener.accept().await {
                Ok((socket, _)) => return Ok(socket),
                Err(err) => {
                    if backoff > BACKOFF_MAX {
                        return Err(err.into());
                    }
                }
            }
            time::sleep(Duration::from_secs(backoff)).await;
            backoff *= 2;
        }
    }
}

struct Handler {
    db: Db,
    connection: Connection,
    shutdown: Shutdown,
    _shutdown_complete: mpsc::Sender<()>,
}

impl Handler {
    fn new(
        db: Db,
        socket: TcpStream,
        shutdown: Shutdown,
        shutdown_complete: mpsc::Sender<()>,
    ) -> Handler {
        Handler {
            db: db,
            connection: Connection::new(socket),
            shutdown: shutdown,
            _shutdown_complete: shutdown_complete,
        }
    }

    #[instrument(skip(self))]
    async fn run(&mut self) -> crate::Result<()> {
        while !self.shutdown.is_shutdown() {
            let option_frame = tokio::select! {
                res = self.connection.read_frame() => res?,
                _ = self.shutdown.recv() => {
                    return Ok(());
                }
            };
            let frame = match option_frame {
                Some(frame) => frame,
                None => return Ok(()),
            };
            let cmd = Command::from_frame(frame)?;
            debug!(?cmd);
            cmd.apply(&self.db, &mut self.connection, &mut self.shutdown)
                .await?;
        }
        Ok(())
    }
}
