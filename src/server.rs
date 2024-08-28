use std::{io::Write, net::SocketAddr};

use tokio::{io::AsyncReadExt, net::TcpListener};

pub struct Server<W: Write> {
    addr: SocketAddr,

    /// Buffer for data to be written too. This buffer sink is for the actual
    /// data that is being sent and _not_ included with log lines.
    buffer: W,
}

impl<W: Write> Server<W> {
    pub fn new(addr: SocketAddr, buffer: W) -> Self {
        Self { addr, buffer }
    }

    pub async fn serve(&mut self) -> crate::Result<()> {
        let bind = TcpListener::bind(self.addr).await?;
        eprintln!("Listening on tcp://{}", bind.local_addr()?);

        while let Ok((mut stream, _addr)) = bind.accept().await {
            let mut s = String::new();
            match stream.read_to_string(&mut s).await {
                Ok(_) => writeln!(self.buffer, "{s}")?,
                Err(e) => eprintln!("Unable to read stream: {e}"),
            }
        }
        unreachable!("This is a blocking call");
    }
}
