mod manager;
mod protocol;
mod server;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub use manager::{SocketManager, WriteOptions};
pub use protocol::Protocol;
pub use server::Server;
