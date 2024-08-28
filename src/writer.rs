use std::net::ToSocketAddrs;

use tokio::{
    io::AsyncWriteExt,
    net::TcpStream,
};

pub struct StreamWriter<'a, S: ToSocketAddrs> {
    host: S,
    input: &'a [u8],

    count: u64,
    bytes_written: u64,
    duration: Option<humantime::Duration>,
}

impl<'a, S> StreamWriter<'a, S>
where
    S: ToSocketAddrs,
{
    pub fn new(
        host: S,
        input: &'a [u8],
        count: u64,
        duration: Option<humantime::Duration>,
    ) -> Self {
        Self {
            host,
            input,
            count,
            duration,
            bytes_written: 0,
        }
    }

    pub async fn write(&mut self) -> crate::Result<u64> {
        let addrs = self
            .host
            .to_socket_addrs()
            .expect("Valid socket addresses are provided");
        for addr in addrs {
            for _ in 0..self.count {
                let mut stream = TcpStream::connect(addr).await?;
                stream.write_all(self.input).await?;
                self.bytes_written += self.input.len() as u64;
            }
        }
        Ok(self.bytes_written)
    }
}
