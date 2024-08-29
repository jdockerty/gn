use std::io::Write;
use std::net::SocketAddr;

use clap::{Parser, Subcommand};
use clap_stdin::MaybeStdin;
use gn::{Server, StreamWriter, WriteOptions};

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
async fn main() -> gn::Result<()> {
    let mut out = std::io::stderr().lock();

    match App::parse().cmds {
        Commands::Write {
            input,
            host,
            count,
            duration,
        } => {
            let opts = match duration {
                Some(d) => WriteOptions::Duration(d),
                None => WriteOptions::Count(count),
            };
            let mut writer = StreamWriter::new(host, input.as_bytes(), opts);
            let wrote = writer.write().await?;
            let throughput = writer.throughput();
            writeln!(out, "Wrote {wrote} bytes")?;
            writeln!(out, "Bytes per second {throughput}")?;
        }
        Commands::Serve { address } => {
            let mut server = Server::new(address, out);
            server.serve().await?;
        }
    };
    Ok(())
}
