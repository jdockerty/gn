use std::io::Write;
use std::net::SocketAddr;

use clap::{Parser, Subcommand};
use clap_stdin::MaybeStdin;
use gn::{statistics::Statistics, Protocol, Server, SocketManager, WriteOptions};

#[derive(Parser)]
struct App {
    #[clap(subcommand)]
    cmds: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Write data over a socket.
    Write {
        #[arg(long)]
        host: SocketAddr,

        #[arg(long, short, default_value = "tcp")]
        protocol: Protocol,

        /// Input data to be written to the socket.
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

        /// Display statistics about writes
        #[clap(long)]
        stats: bool,
    },
    /// Start a server, listening for a specified protocol.
    Serve {
        #[arg(long, default_value = "127.0.0.1:5000")]
        address: SocketAddr,

        #[arg(long, short, default_value = "tcp")]
        protocol: Protocol,
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
            stats,
        } => {
            let opts = WriteOptions::from_flags(count, duration, concurrency);
            let statistics = Statistics::new();
            let mut manager =
                SocketManager::new(host, input.as_bytes(), protocol, opts, statistics);
            manager.write().await?;

            if stats {
                match manager.elapsed() {
                    0..1000 => writeln!(
                        out,
                        "Sent: {} bytes in {}ms",
                        manager.total_bytes(),
                        manager.elapsed()
                    )?,
                    _ => writeln!(
                        out,
                        "Sent: {} bytes in {}s",
                        manager.total_bytes(),
                        manager.elapsed() / 1000
                    )?,
                }
                writeln!(out, "Throughput: {} bytes per second", manager.throughput())?;
                writeln!(
                    out,
                    "Requests: {}/{} ({:.2}%) successful",
                    manager.successful_requests(),
                    count,
                    manager.successful_requests_percentage()
                )?;
            }
        }
        Commands::Serve { address, protocol } => {
            let mut server = Server::new(address, protocol, out);
            server.serve().await?;
        }
    };
    Ok(())
}
