use std::io::Write;
use std::net::SocketAddr;

use clap::{Parser, Subcommand};
use clap_stdin::MaybeStdin;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream}, time::Instant,
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Parser)]
struct App {
    #[clap(subcommand)]
    cmds: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Write data over a TCP socket.
    Write {
        #[arg(long)]
        host: SocketAddr,

        /// Input data to be written to the TCP socket.
        ///
        /// Defaults to reading from stdin when unspecified.
        #[clap(default_value = "-")]
        input: MaybeStdin<String>,

        #[clap(short, long, default_value = "1")]
        count: u64,

        #[clap(short, long)]
        duration: Option<humantime::Duration>,
    },
    /// Start a TCP server
    Serve {
        #[arg(long, default_value = "127.0.0.1:5000")]
        address: SocketAddr,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut out = std::io::stderr().lock();

    match App::parse().cmds {
        Commands::Write { input, host, count, duration: _ } => {
            let start = Instant::now();
            let mut wrote: u64 = 0;
            for _ in 0..count {
                let mut stream = TcpStream::connect(host).await?;
                let input = input.as_bytes();
                stream.write_all(input).await?;
                wrote += input.len() as u64;
            }
            let elapsed = start.elapsed().as_secs();
            let _throughput = wrote / elapsed;
            writeln!(out, "Wrote {wrote} bytes").unwrap();
        }
        Commands::Serve { address } => {
            let bind = TcpListener::bind(address).await?;
            writeln!(out, "Listening on tcp://{}", bind.local_addr()?)?;

            while let Ok((mut stream, _addr)) = bind.accept().await {
                let mut s = String::new();
                match stream.read_to_string(&mut s).await {
                    Ok(_) => writeln!(out, "{s}")?,
                    Err(e) => writeln!(out, "Unable to read stream: {e}")?,
                }
            }
        }
    };
    Ok(())
}
