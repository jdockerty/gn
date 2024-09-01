use std::io::Write;
use std::net::SocketAddr;

use clap::{Parser, Subcommand};
use clap_stdin::MaybeStdin;
use gn::{Protocol, Server, SocketManager, WriteOptions};

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

        #[arg(long, short, default_value = "tcp")]
        protocol: Protocol,

        /// Input data to be written to the TCP socket.
        ///
        /// Defaults to reading from stdin when unspecified.
        #[clap(default_value = "-")]
        input: MaybeStdin<String>,

        #[clap(short, long, default_value = "1")]
        count: u64,

        /// The duration of time to write for, e.g. 30s
        ///
        /// When provided alongside `count`, whichever comes first will then halt
        /// writes. For example, duration=40s and count=10, the count is in almost
        /// every case going to be reached first.
        #[clap(short, long)]
        duration: Option<humantime::Duration>,

        /// Number of concurrent requests to send.
        #[clap(long)]
        concurrency: Option<u64>,
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
            concurrency,
            protocol,
        } => {
            let opts = WriteOptions::from_flags(count, duration, concurrency);
            let mut writer = SocketManager::new(host, input.as_bytes(), protocol, opts);
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
