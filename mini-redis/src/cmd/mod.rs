use std::time::Duration;
use bytes::Bytes;
use crate::Frame;
use crate::Error;

#[derive(Debug)]
pub enum Command {
    Get(Get),
    Publish(Publish),
    Set(Set),
    Subscribe(Subscribe),
    Unsubscribe(Unsubscribe),
    Ping(Ping),
    Unknown(Unknown),
}

impl Command {
    pub fn from_frame(frame: Frame) -> crate::Result<Command> {
        let command = Command::Get(Get{key:"key".to_string()});
        Ok(command)
    }
}

#[derive(Debug)]
pub struct Get {
    key: String,
}

#[derive(Debug)]
pub struct Publish {
    channel: String,
    message: Bytes,
}

#[derive(Debug)]
pub struct Set {
    key: String,
    value: Bytes,
    expire: Option<Duration>,
}

#[derive(Debug)]
pub struct Subscribe {
    channels: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct Unsubscribe {
    channels: Vec<String>,
}

#[derive(Debug, Default)]
pub struct Ping {
    msg: Option<Bytes>,
}

#[derive(Debug)]
pub struct Unknown {
    command_name: String,
}