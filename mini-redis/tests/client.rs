use bytes::Bytes;
use mini_redis::{clients::Client, server};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

async fn start_server() -> (SocketAddr, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handler = tokio::spawn(async move {
        server::run(listener, tokio::signal::ctrl_c()).await;
    });
    (addr, handler)
}

async fn start_server_client() -> Client {
    let (addr, _) = start_server().await;
    let client = Client::connect(addr).await.unwrap();
    client
}

#[tokio::test]
async fn ping_pong_without_message() {
    let mut client = start_server_client().await;
    let pong = client.ping(None).await.unwrap();
    assert_eq!(b"PONG", &pong[..]);
}

#[tokio::test]
async fn ping_pong_with_message() {
    let mut client = start_server_client().await;
    let pong = client.ping(Some(Bytes::from("hello world"))).await.unwrap();
    assert_eq!(b"hello world", &pong[..]);
}

#[tokio::test]
async fn key_value_get_set() {
    let mut client = start_server_client().await;
    client.set("hello", "world".into()).await.unwrap();
    let got = client.get("hello").await.unwrap().unwrap();
    assert_eq!(b"world", &got[..]);
}

#[tokio::test]
async fn receive_message_subscribed_channel() {
    let (addr, _) = start_server().await;
    let client = Client::connect(addr).await.unwrap();
    let mut subscriber = client.subscribe(vec!["hello".into()]).await.unwrap();
    tokio::spawn(async move {
        let mut client = Client::connect(addr).await.unwrap();
        client.publish("hello", "world".into()).await.unwrap();
    });
    let message = subscriber.next_message().await.unwrap().unwrap();
    assert_eq!("hello", &message.channel);
    assert_eq!(b"world", &message.content[..]);
}

#[tokio::test]
async fn unsubscribes_from_channels_specific() {
    let (addr, _) = start_server().await;

    let client = Client::connect(addr).await.unwrap();
    let mut subscriber = client
        .subscribe(vec!["hello".into(), "world".into()])
        .await
        .unwrap();

    subscriber.unsubscribe(&vec!["hello".into()]).await;
    assert_eq!(subscriber.get_subscribed().len(), 1);
    assert_eq!(subscriber.get_subscribed()[0], "world");
}

#[tokio::test]
async fn unsubscribes_from_channels() {
    let (addr, _) = start_server().await;

    let client = Client::connect(addr).await.unwrap();
    let mut subscriber = client
        .subscribe(vec!["hello".into(), "world".into()])
        .await
        .unwrap();

    subscriber.unsubscribe(&[]).await;
    assert_eq!(subscriber.get_subscribed().len(), 0);
}
