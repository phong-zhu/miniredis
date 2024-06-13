use crate::parse::ParseError;
use crate::Db;
use crate::Parse;
use crate::Shutdown;
use crate::{Connection, Frame};
use bytes::Bytes;
use clap::{Parser, Subcommand};
use std::pin::Pin;
use std::time::Duration;
use tokio::select;
use tokio::sync::broadcast;
use tokio_stream::{Stream, StreamExt, StreamMap};
use tracing::debug;

#[derive(Debug)]
pub enum Command {
    Get(Get),
    Set(Set),
    Publish(Publish),
    Subscribe(Subscribe),
    Unsubscribe(Unsubscribe),
    Ping(Ping),
    Unknown(Unknown),
}

impl Command {
    pub fn from_frame(frame: Frame) -> crate::Result<Command> {
        let mut parse = Parse::new(frame)?;
        let command_name = parse.next_string()?.to_lowercase();
        let command = match &command_name[..] {
            "get" => Command::Get(Get::parse_frames(&mut parse)?),
            "publish" => Command::Publish(Publish::parse_frames(&mut parse)?),
            "set" => Command::Set(Set::parse_frames(&mut parse)?),
            "subscribe" => Command::Subscribe(Subscribe::parse_frames(&mut parse)?),
            "unsubscribe" => Command::Unsubscribe(Unsubscribe::parse_frames(&mut parse)?),
            "ping" => Command::Ping(Ping::parse_frames(&mut parse)?),
            _ => return Ok(Command::Unknown(Unknown::new(command_name))),
        };
        parse.finish()?;
        Ok(command)
    }

    pub fn get_name(&self) -> &str {
        match self {
            Command::Get(_) => "get",
            Command::Set(_) => "set",
            Command::Publish(_) => "publish",
            Command::Subscribe(_) => "subscribe",
            Command::Unsubscribe(_) => "unsubscribe",
            Command::Ping(_) => "ping",
            Command::Unknown(cmd) => cmd.get_name(),
        }
    }

    pub async fn apply(
        self,
        db: &Db,
        connection: &mut Connection,
        shutdown: &mut Shutdown,
    ) -> crate::Result<()> {
        use Command::*;
        match self {
            Get(cmd) => cmd.apply(db, connection).await,
            Set(cmd) => cmd.apply(db, connection).await,
            Publish(cmd) => cmd.apply(db, connection).await,
            Subscribe(cmd) => cmd.apply(db, connection, shutdown).await,
            Ping(cmd) => cmd.apply(db, connection).await,
            Unknown(cmd) => cmd.apply(connection).await,
            // `Unsubscribe` cannot be applied. It may only be received from the
            // context of a `Subscribe` command.
            Unsubscribe(_) => Err("`Unsubscribe` is unsupported in this context".into()),
        }
    }
}

#[derive(Debug)]
pub struct Get {
    key: String,
}

impl Get {
    pub fn new(key: impl ToString) -> Get {
        Get {
            key: key.to_string(),
        }
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn parse_frames(parse: &mut Parse) -> crate::Result<Get> {
        let key = parse.next_string()?;
        Ok(Get { key })
    }

    pub fn into_frame(self) -> Frame {
        let mut frame = Frame::array();
        frame.push_bulk(Bytes::from("get".as_bytes()));
        frame.push_bulk(Bytes::from(self.key.into_bytes()));
        frame
    }

    pub async fn apply(self, db: &Db, connection: &mut Connection) -> crate::Result<()> {
        let frame = if let Some(value) = db.get(&self.key) {
            Frame::Bulk(value)
        } else {
            Frame::Null
        };
        debug!(?frame);
        connection.write_frame(&frame).await?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct Publish {
    channel: String,
    message: Bytes,
}

impl Publish {
    pub fn new(channel: impl ToString, message: Bytes) -> Publish {
        Publish {
            channel: channel.to_string(),
            message,
        }
    }

    pub fn parse_frames(parse: &mut Parse) -> crate::Result<Publish> {
        let channel = parse.next_string()?;
        let message = parse.next_bytes()?;
        Ok(Publish { channel, message })
    }

    pub fn into_frame(self) -> Frame {
        let mut frame = Frame::array();
        frame.push_bulk(Bytes::from("publish".as_bytes()));
        frame.push_bulk(Bytes::from(self.channel.into_bytes()));
        frame.push_bulk(Bytes::from(self.message));
        frame
    }

    pub async fn apply(self, db: &Db, connection: &mut Connection) -> crate::Result<()> {
        let num_subscribers = db.publish(&self.channel, self.message);
        let response = Frame::Integer(num_subscribers as u64);
        connection.write_frame(&response).await?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct Set {
    key: String,
    value: Bytes,
    expire: Option<Duration>,
}

impl Set {
    pub fn new(key: impl ToString, value: Bytes, expire: Option<Duration>) -> Set {
        Set {
            key: key.to_string(),
            value,
            expire,
        }
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn value(&self) -> &Bytes {
        &self.value
    }

    pub fn expire(&self) -> Option<Duration> {
        self.expire
    }

    pub fn into_frame(self) -> Frame {
        let mut frame = Frame::array();
        frame.push_bulk(Bytes::from("set".as_bytes()));
        frame.push_bulk(Bytes::from(self.key.into_bytes()));
        frame.push_bulk(self.value);
        if let Some(ms) = self.expire {
            frame.push_bulk(Bytes::from("px".as_bytes()));
            frame.push_int(ms.as_millis() as u64);
        }
        frame
    }

    pub fn parse_frames(parse: &mut Parse) -> crate::Result<Set> {
        use ParseError::EndOfStream;
        let key = parse.next_string()?;
        let value = parse.next_bytes()?;
        let mut expire = None;
        match parse.next_string() {
            Ok(s) if s.to_uppercase() == "EX" => {
                let sec = parse.next_int()?;
                expire = Some(Duration::from_secs(sec));
            }
            Ok(s) if s.to_uppercase() == "PX" => {
                let ms = parse.next_int()?;
                expire = Some(Duration::from_millis(ms));
            }
            Ok(_) => return Err("SET only supports expire option".into()),
            Err(EndOfStream) => {}
            Err(err) => return Err(err.into()),
        }
        Ok(Set {
            key: key.to_string(),
            value,
            expire,
        })
    }

    pub async fn apply(self, db: &Db, connection: &mut Connection) -> crate::Result<()> {
        db.set(self.key, self.value, self.expire);
        let frame = Frame::Simple("OK".to_string());
        debug!(?frame);
        connection.write_frame(&frame).await?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct Subscribe {
    channels: Vec<String>,
}

impl Subscribe {
    pub fn new(channels: Vec<String>) -> Subscribe {
        Subscribe { channels }
    }

    pub fn parse_frames(parse: &mut Parse) -> crate::Result<Subscribe> {
        use ParseError::EndOfStream;
        let mut channels = vec![parse.next_string()?];
        loop {
            match parse.next_string() {
                Ok(s) => channels.push(s),
                Err(EndOfStream) => break,
                Err(err) => return Err(err.into()),
            }
        }
        Ok(Subscribe { channels })
    }

    pub fn into_frame(self) -> Frame {
        let mut frame = Frame::array();
        frame.push_bulk(Bytes::from("subscribe".as_bytes()));
        for channel in self.channels {
            frame.push_bulk(Bytes::from(channel.into_bytes()));
        }
        frame
    }

    pub async fn apply(
        mut self,
        db: &Db,
        connection: &mut Connection,
        shutdown: &mut Shutdown,
    ) -> crate::Result<()> {
        // let f = Frame::Simple("test".to_string());
        // connection.write_frame(&f).await?;
        // Ok(())
        let mut subscriptions = StreamMap::new();
        loop {
            for channel_name in self.channels.drain(..) {
                subscribe_to_channel(channel_name, &mut subscriptions, db, connection).await?;
            }
            select! {
                Some((channel_name, msg)) = subscriptions.next() => {
                   connection.write_frame(&make_message_frame(channel_name, msg)).await?;
                }
                res = connection.read_frame() => {
                    let frame = match res? {
                        Some(frame) => frame,
                        // This happens if the remote client has disconnected.
                        None => return Ok(())
                    };

                    handle_command(
                        frame,
                        &mut self.channels,
                        &mut subscriptions,
                        connection,
                    ).await?;
                }
                _ = shutdown.recv() => {
                    return Ok(());
                }
            }
        }
    }
}

type Messages = Pin<Box<dyn Stream<Item = Bytes> + Send>>;

async fn subscribe_to_channel(
    channel_name: String,
    subscriptions: &mut StreamMap<String, Messages>,
    db: &Db,
    connection: &mut Connection,
) -> crate::Result<()> {
    let mut rx = db.subscribe(channel_name.clone());
    let rx = Box::pin(async_stream::stream! {
        loop {
            match rx.recv().await {
                Ok(msg) => yield msg,
                // If we lagged in consuming messages, just resume.
                Err(broadcast::error::RecvError::Lagged(_)) => {}
                Err(_) => break,
            }
        }
    });
    subscriptions.insert(channel_name.clone(), rx);
    let response = make_subscribe_frame(channel_name, subscriptions.len());
    connection.write_frame(&response).await?;
    Ok(())
}

fn make_subscribe_frame(channel_name: String, num_subs: usize) -> Frame {
    let mut response = Frame::array();
    response.push_bulk(Bytes::from_static(b"subscribe"));
    response.push_bulk(Bytes::from(channel_name));
    response.push_int(num_subs as u64);
    response
}

fn make_unsubscribe_frame(channel_name: String, num_subs: usize) -> Frame {
    let mut response = Frame::array();
    response.push_bulk(Bytes::from_static(b"unsubscribe"));
    response.push_bulk(Bytes::from(channel_name));
    response.push_int(num_subs as u64);
    response
}

fn make_message_frame(channel_name: String, msg: Bytes) -> Frame {
    let mut response = Frame::array();
    response.push_bulk(Bytes::from_static(b"message"));
    response.push_bulk(Bytes::from(channel_name));
    response.push_bulk(msg);
    response
}

async fn handle_command(
    frame: Frame,
    subscribe_to: &mut Vec<String>,
    subscriptions: &mut StreamMap<String, Messages>,
    connection: &mut Connection,
) -> crate::Result<()> {
    match Command::from_frame(frame)? {
        Command::Subscribe(subscribe) => {
            subscribe_to.extend(subscribe.channels.into_iter());
        }
        Command::Unsubscribe(mut unsubscribe) => {
            if unsubscribe.channels.is_empty() {
                if unsubscribe.channels.is_empty() {
                    unsubscribe.channels = subscriptions
                        .keys()
                        .map(|channel_name| channel_name.to_string())
                        .collect();
                }
                for channel_name in unsubscribe.channels {
                    subscriptions.remove(&channel_name);
                    let response = make_unsubscribe_frame(channel_name, subscriptions.len());
                    connection.write_frame(&response).await?;
                }
            }
        }
        command => {
            let cmd = Unknown::new(command.get_name());
            cmd.apply(connection).await?;
        }
    }
    Ok(())
}

#[derive(Clone, Debug)]
pub struct Unsubscribe {
    channels: Vec<String>,
}

impl Unsubscribe {
    pub fn new(channels: &[String]) -> Unsubscribe {
        Unsubscribe {
            channels: channels.to_vec(),
        }
    }

    pub fn parse_frames(parse: &mut Parse) -> crate::Result<Unsubscribe> {
        use ParseError::EndOfStream;
        let mut channels = vec![];
        loop {
            match parse.next_string() {
                Ok(s) => channels.push(s),
                Err(EndOfStream) => break,
                Err(err) => return Err(err.into()),
            }
        }
        Ok(Unsubscribe { channels })
    }

    pub fn into_frame(self) -> Frame {
        let mut frame = Frame::array();
        frame.push_bulk(Bytes::from("unsubscribe".as_bytes()));
        for channel in self.channels {
            frame.push_bulk(Bytes::from(channel.into_bytes()));
        }
        frame
    }
}

#[derive(Debug, Default)]
pub struct Ping {
    msg: Option<Bytes>,
}

impl Ping {
    pub fn new(msg: Option<Bytes>) -> Ping {
        Ping { msg }
    }

    pub fn parse_frames(parse: &mut Parse) -> crate::Result<Ping> {
        match parse.next_bytes() {
            Ok(msg) => Ok(Ping::new(Some(msg))),
            Err(ParseError::EndOfStream) => Ok(Ping::default()),
            Err(e) => Err(e.into()),
        }
    }

    pub fn into_frame(self) -> Frame {
        let mut frame = Frame::array();
        frame.push_bulk(Bytes::from("ping".as_bytes()));
        if let Some(msg) = self.msg {
            frame.push_bulk(msg);
        }
        frame
    }

    pub async fn apply(self, db: &Db, connection: &mut Connection) -> crate::Result<()> {
        let frame = Frame::Bulk(self.get_msg());
        connection.write_frame(&frame).await?;
        Ok(())
    }

    // pub fn get_msg(self) -> Option<Bytes> {
    //     if let Some(msg) = self.msg {
    //         Some(msg)
    //     } else {
    //         Some("PONG".as_bytes().into())
    //     }
    // }
    pub fn get_msg(self) -> Bytes {
        if let Some(msg) = self.msg {
            msg
        } else {
            "PONG".as_bytes().into()
        }
    }
}

#[derive(Debug)]
pub struct Unknown {
    command_name: String,
}

impl Unknown {
    pub fn new(key: impl ToString) -> Unknown {
        Unknown {
            command_name: key.to_string(),
        }
    }

    pub fn get_name(&self) -> &str {
        &self.command_name
    }

    pub async fn apply(self, connection: &mut Connection) -> crate::Result<()> {
        let frame = Frame::Error(format!("ERR unknown command {}", self.command_name));
        debug!(?frame);
        connection.write_frame(&frame).await?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_get() {
        let get = Get::new("foo");
        assert_eq!(get.key(), "foo".to_string());

        let frame = get.into_frame();
        let cmd = Command::from_frame(frame).unwrap();
        let got = match cmd {
            Command::Get(got) => got,
            _ => panic!("not match"),
        };
        assert_eq!(got.key(), "foo");
    }
}
