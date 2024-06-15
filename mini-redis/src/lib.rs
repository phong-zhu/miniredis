pub mod clients;
pub mod cmd;
pub mod frame;
pub mod server;

mod connection;
mod db;
mod parse;
mod shutdown;

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, Error>;

pub use clients::BlockingClient;
pub use clients::BufferedClient;
pub use clients::Client;
pub use cmd::Command;
pub use connection::Connection;
pub use db::Db;
pub use frame::Frame;
pub use server::run;
pub use shutdown::Shutdown;

use parse::{Parse, ParseError};
