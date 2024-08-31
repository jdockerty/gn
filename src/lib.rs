mod manager;
mod server;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub use manager::{SocketManager, WriteOptions};
pub use server::Server;
