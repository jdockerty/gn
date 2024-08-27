use std::net::SocketAddr;

use clap::{Parser, Subcommand};
use clap_stdin::FileOrStdin;
use tokio::{io::AsyncReadExt, net::TcpListener};

#[derive(Parser)]
struct App {
    #[clap(subcommand)]
    cmds: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Write data over a TCP socket
    Write {
        /// Data to be written
        input: FileOrStdin,
    },
    /// Start a TCP server
    Serve {
        #[arg(long, default_value = "127.0.0.1:5000")]
        address: SocketAddr,
    },
}

#[tokio::main]
async fn main() {
    match App::parse().cmds {
        Commands::Write { input: _ } => {}
        Commands::Serve { address } => {
            let bind = TcpListener::bind(address).await.unwrap();
            eprintln!("Listening on tcp://{}", bind.local_addr().unwrap());

            while let Ok((mut stream, _addr)) = bind.accept().await {
                let mut s = String::new();
                match stream.read_to_string(&mut s).await {
                    Ok(_) => eprintln!("{s}"),
                    Err(e) => eprintln!("Unable to read stream: {e}"),
                }
            }
        }
    };
}
