use std::{io::Write, net::SocketAddr};

use tokio::{
    io::AsyncReadExt,
    net::{TcpListener, UdpSocket},
};

use crate::Protocol;

pub struct Server<W: Write> {
    addr: SocketAddr,
    protocol: Protocol,

    /// Buffer for data to be written too. This buffer sink is for the actual
    /// data that is being sent and _not_ included with log lines.
    buffer: W,
}

impl<W: Write> Server<W> {
    pub fn new(addr: SocketAddr, protocol: Protocol, buffer: W) -> Self {
        Self {
            addr,
            protocol,
            buffer,
        }
    }

    pub async fn serve(&mut self) -> crate::Result<()> {
        match self.protocol {
            Protocol::Tcp => {
                let bind = TcpListener::bind(self.addr).await?;
                eprintln!("Listening on tcp://{}", bind.local_addr()?);

                while let Ok((mut stream, _addr)) = bind.accept().await {
                    let mut s = String::new();
                    match stream.read_to_string(&mut s).await {
                        Ok(_) => writeln!(self.buffer, "{s}")?,
                        Err(e) => eprintln!("Unable to read stream: {e}"),
                    }
                }
            }
            Protocol::Udp => {
                let bind = UdpSocket::bind(self.addr).await?;
                eprintln!("Listening on udp://{}", bind.local_addr()?);
                loop {
                    let mut buf = [0; 1024];
                    while let Ok((len, _addr)) = bind.recv_from(&mut buf).await {
                        writeln!(self.buffer, "{}", String::from_utf8_lossy(&buf[0..len]))?;
                    }
                }
            }
        }
        unreachable!("This is a blocking call");
    }
}
