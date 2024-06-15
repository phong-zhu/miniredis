use mini_redis::{clients::Client, server};
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinHandle;

async fn start_server() -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        server::run(listener, tokio::signal::ctrl_c()).await;
    });
    addr
}

#[tokio::test]
async fn key_value_get_set_svr() {
    let addr = start_server().await;
    let mut stream = TcpStream::connect(addr).await.unwrap();

    let response = get_hello_raw(&mut stream, 5).await;
    assert_eq!(b"$-1\r\n", &response[..]);

    stream
        .write_all(b"*3\r\n$3\r\nSET\r\n$5\r\nhello\r\n$5\r\nworld\r\n")
        .await
        .unwrap();
    let mut response = [0; 5];
    stream.read_exact(&mut response).await.unwrap();
    assert_eq!(b"+OK\r\n", &response);

    let response = get_hello_raw(&mut stream, 11).await;
    assert_eq!(b"$5\r\nworld\r\n", &response[..]);
}

#[tokio::test]
async fn send_error_unknown_command() {
    let addr = start_server().await;
    let mut stream = TcpStream::connect(addr).await.unwrap();
    stream
        .write_all(b"*2\r\n$3\r\nFOO\r\n$5\r\nhello\r\n")
        .await
        .unwrap();
    let mut response = [0; 28];
    let len = stream.read(&mut response).await.unwrap();
    println!(
        "read buf {:?}",
        String::from_utf8(Vec::from(&response[0..len]))
    );
    assert_eq!(b"-ERR unknown command foo\r\n", &response[0..len]);
}

async fn get_hello_raw(stream: &mut TcpStream, size: usize) -> Vec<u8> {
    stream
        .write_all(b"*2\r\n$3\r\nGET\r\n$5\r\nhello\r\n")
        .await
        .unwrap();
    let mut response = vec![0; size];
    let _ = match stream.read_exact(&mut response).await {
        Ok(len) => len,
        Err(err) => panic!("read err: {}", err),
    };
    response
}
