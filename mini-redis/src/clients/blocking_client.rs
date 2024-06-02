use crate::clients::{Client, Subscriber};
use crate::Result;
use bytes::Bytes;
use tokio::{net::ToSocketAddrs, runtime::Runtime};

use super::Message;

pub struct BlockingClient {
    inner: Client,
    rt: Runtime,
}

impl BlockingClient {
    pub fn connect<T: ToSocketAddrs>(addr: T) -> Result<BlockingClient> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        let inner = rt.block_on(Client::connect(addr))?;
        Ok(BlockingClient { inner, rt })
    }

    pub fn get(&mut self, key: &str) -> Result<Option<Bytes>> {
        self.rt.block_on(self.inner.get(key))
    }

    pub fn set(&mut self, key: &str, value: Bytes) -> Result<()> {
        self.rt.block_on(self.inner.set(key, value))
    }

    pub fn publish(&mut self, channel: &str, message: Bytes) -> Result<u64> {
        self.rt.block_on(self.inner.publish(channel, message))
    }

    pub fn subscribe(self, channels: Vec<String>) -> Result<BlockingSubscriber> {
        let subscriber = self.rt.block_on(self.inner.subscribe(channels))?;
        Ok(BlockingSubscriber {
            inner: subscriber,
            rt: self.rt,
        })
    }
}

pub struct BlockingSubscriber {
    inner: Subscriber,
    rt: Runtime,
}

impl BlockingSubscriber {
    pub fn get_subscribed(&self) -> &[String] {
        self.inner.get_subscribed()
    }

    pub fn next_message(&mut self) -> Result<Option<Message>> {
        self.rt.block_on(self.inner.next_message())
    }

    pub fn into_iter(self) -> impl Iterator<Item = Result<Message>> {
        SubscriberIterator {
            inner: self.inner,
            rt: self.rt,
        }
    }

    pub fn subscirbe(&mut self, channels: &[String]) -> Result<()> {
        self.rt.block_on(self.inner.subscribe(channels))
    }
}

struct SubscriberIterator {
    inner: Subscriber,
    rt: Runtime,
}

impl Iterator for SubscriberIterator {
    type Item = Result<Message>;

    fn next(&mut self) -> Option<Self::Item> {
        self.rt.block_on(self.inner.next_message()).transpose()
    }
}
