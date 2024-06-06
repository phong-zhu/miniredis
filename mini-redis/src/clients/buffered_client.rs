use bytes::Bytes;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::oneshot;

use crate::clients::Client;
use crate::cmd::{Command, Get, Set};
use crate::Result;

type Message = (Command, oneshot::Sender<Result<Option<Bytes>>>);

pub struct BufferedClient {
    tx: Sender<Message>,
}

impl BufferedClient {
    pub fn buffer(client: Client) -> BufferedClient {
        let (tx, rx) = channel(32);
        tokio::spawn(async move { run(client, rx).await });
        BufferedClient { tx }
    }

    pub async fn get(&mut self, key: &str) -> Result<Option<Bytes>> {
        let get = Command::Get(Get::new(key));
        let (tx, rx) = oneshot::channel();
        self.tx.send((get, tx)).await?;
        match rx.await {
            Ok(res) => res,
            Err(err) => Err(err.into()),
        }
    }

    pub async fn set(&mut self, key: &str, value: Bytes) -> Result<()> {
        let set = Command::Set(Set::new(key, value, None));
        let (tx, rx) = oneshot::channel();
        self.tx.send((set, tx)).await?;
        match rx.await {
            Ok(res) => res.map(|_| ()),
            Err(err) => Err(err.into()),
        }
    }
}

async fn run(mut client: Client, mut rx: Receiver<Message>) {
    while let Some((cmd, tx)) = rx.recv().await {
        let response = match cmd {
            Command::Get(cmd) => client.get(cmd.key()).await,
            Command::Set(cmd) => client
                .set(cmd.key(), cmd.value().clone())
                .await
                .map(|_| None),
            _ => Err("unimplemented".into()),
        };
        let _ = tx.send(response);
    }
}
