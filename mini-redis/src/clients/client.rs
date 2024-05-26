use std::io::{Error, ErrorKind};
use bytes::Bytes;
use tokio::net::{TcpStream, ToSocketAddrs};
use tokio_stream::{Stream};
use async_stream::try_stream;
use tracing::{debug, instrument};
use crate::cmd::{Get, Ping, Publish, Subscribe};
use crate::{Connection, Frame};

pub struct Client {
    connection: Connection,
}

impl Client {
    pub async fn connect<T: ToSocketAddrs>(addr: T) -> crate::Result<Client> {
        let socket = TcpStream::connect(addr).await?;
        let connection = Connection::new(socket);
        Ok(Client{connection})
    }

    #[instrument(skip(self))]
    pub async fn get(&mut self, key: &str) -> crate::Result<Option<Bytes>> {
        let frame = Get::new(key).into_frame();
        self.connection.write_frame(&frame).await?;
        match self.read_response().await? {
            Frame::Simple(value) => Ok(Some(value.into())),
            Frame::Bulk(value) => Ok(Some(value)),
            Frame::Null => Ok(None),
            frame => Err(frame.to_error()),
        }
    }

    pub async fn ping(&mut self, msg: Option<Bytes>) -> crate::Result<Bytes> {
        let frame = Ping::new(msg).into_frame();
        debug!(request = ?frame);
        self.connection.write_frame(&frame).await?;

        match self.read_response().await? {
            Frame::Simple(value) => Ok(value.into()),
            Frame::Bulk(value) => Ok(value),
            frame => Err(frame.to_error()),
        }
    }

    pub async fn publish(&mut self, channel: &str, message: Bytes) -> crate::Result<u64> {
        let frame = Publish::new(channel, message).into_frame();
        debug!(request = ?frame);
        self.connection.write_frame(&frame).await?;
        match self.read_response().await? {
            Frame::Integer(response) => Ok(response),
            frame => Err(frame.to_error()),
        }
    }

    pub async fn subscribe(mut self, channels: Vec<String>) -> crate::Result<Subscriber> {
        self.subscribe_cmd(&channels).await?;
        Ok(Subscriber{
            client: self,
            subscribed_channels: channels,
        })
    }

    async fn subscribe_cmd(&mut self, channels: &[String]) -> crate::Result<()> {
        let frame = Subscribe::new(channels.to_vec()).into_frame();
        debug!(request = ?frame);
        self.connection.write_frame(&frame).await?;
        for channel in channels {
            let response = self.read_response().await?;
            match response {
                Frame::Array(ref frame) => match frame.as_slice() {
                    [subscribe, schannel, ..] if *subscribe == "subscribe" && *schannel == channel => {}
                    _ => return Err(response.to_error()),
                }
                frame => return Err(frame.to_error()),
            }
        }
        Ok(())
    }

    async fn read_response(&mut self) -> crate::Result<Frame> {
        let response = self.connection.read_frame().await?;
        debug!(?response);
        match response {
            Some(Frame::Error(msg)) => Err(msg.into()),
            Some(frame) => Ok(frame),
            None => {
                let err = Error::new(ErrorKind::ConnectionReset, "reset by peer");
                Err(err.into())
            }
        }
    }
}

pub struct Subscriber {
    client: Client,
    subscribed_channels: Vec<String>,
}

impl Subscriber {
    pub fn get_subscribed(&self) -> &[String] {
        &self.subscribed_channels
    }

    pub async fn next_message(&mut self) -> crate::Result<Option<Message>> {
        match self.client.connection.read_frame().await? {
            Some(mframe) => {
                debug!(?mframe);
                match mframe {
                    Frame::Array(ref frame) => match frame.as_slice() {
                        [message, channel, content] if *message == "message"
                        => Ok(Some(Message{
                            channel: channel.to_string(),
                            content: Bytes::from(content.to_string()),
                        })),
                        _ => Err(mframe.to_error()),
                    },
                    frame => Err(frame.to_error()),
                }
            }
            None => Ok(None),
        }
    }

    pub fn into_stream(mut self) -> impl Stream<Item = crate::Result<Message>> {
        try_stream! {
            while let Some(message)= self.next_message().await? {
                yield message;
            }
        }
    }
}

pub struct Message {
    pub channel: String,
    pub content: Bytes,
}

impl Message {
}