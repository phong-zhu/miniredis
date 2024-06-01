mod clients;
mod cmd;
mod connection;
mod db;
mod frame;
mod parse;
mod server;
mod shutdown;

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, Error>;

pub use clients::Client;
pub use cmd::Command;
pub use connection::Connection;
pub use db::Db;
pub use frame::Frame;
pub use server::run;
pub use shutdown::Shutdown;

use parse::{Parse, ParseError};

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
