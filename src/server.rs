use std::{io::Write, net::SocketAddr};

use tokio::{io::AsyncReadExt, net::TcpListener};

pub struct Server<W: Write> {
    addr: SocketAddr,
    out: W,
}

impl<W: Write> Server<W> {
    pub fn new(addr: SocketAddr, out: W) -> Self {
        Self { addr, out }
    }

    pub async fn serve(&mut self) -> crate::Result<()> {
        let bind = TcpListener::bind(self.addr).await?;
        writeln!(self.out, "Listening on tcp://{}", bind.local_addr()?)?;

        while let Ok((mut stream, _addr)) = bind.accept().await {
            let mut s = String::new();
            match stream.read_to_string(&mut s).await {
                Ok(_) => writeln!(self.out, "{s}")?,
                Err(e) => writeln!(self.out, "Unable to read stream: {e}")?,
            }
        }

        unreachable!("This is a blocking call");
    }
}
