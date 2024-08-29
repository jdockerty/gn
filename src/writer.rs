use std::net::{SocketAddr, ToSocketAddrs};

use tokio::{io::AsyncWriteExt, net::TcpStream, time::Instant};

pub enum WriteOptions {
    Count(u64),
    Duration(humantime::Duration),
}

pub struct StreamWriter<'a, S: ToSocketAddrs> {
    host: S,
    input: &'a [u8],
    input_size: u64,

    write_options: WriteOptions,

    bytes_written: u64,
    throughput: f64,
}

impl<'a, S> StreamWriter<'a, S>
where
    S: ToSocketAddrs,
{
    pub fn new(host: S, input: &'a [u8], write_options: WriteOptions) -> Self {
        Self {
            host,
            input,
            input_size: input.len() as u64,
            write_options,
            bytes_written: 0,
            throughput: 0.0,
        }
    }

    /// Write the provided input data to a [`SocketAddr`].
    async fn write_stream(&mut self, addr: SocketAddr) -> crate::Result<()> {
        let mut stream = TcpStream::connect(addr).await?;
        stream.write_all(self.input).await?;
        self.bytes_written += self.input_size;
        Ok(())
    }

    /// Write to the provided host(s), returning the total number of bytes written.
    /// At the same time, this also calculates the throughput for total number
    /// of bytes sent per second.
    ///
    /// NOTE: Owing to truncation from nanosecond precision to seconds, the
    /// produced throughput may not be accurate for low write counts.
    pub async fn write(&mut self) -> crate::Result<u64> {
        let addrs = self
            .host
            .to_socket_addrs()
            .expect("Valid socket addresses are provided");
        let start = Instant::now();
        for addr in addrs {
            match self.write_options {
                WriteOptions::Count(count) => {
                    for _ in 0..count {
                        self.write_stream(addr).await?;
                    }
                }
                WriteOptions::Duration(duration) => {
                    let for_duration = Instant::now();
                    loop {
                        if for_duration.elapsed() >= *duration {
                            break;
                        } else {
                            self.write_stream(addr).await?;
                        }
                    }
                }
            }
        }

        self.throughput = self.bytes_written as f64 / start.elapsed().as_secs() as f64;

        Ok(self.bytes_written)
    }

    /// Retrieve the perceived bytes per second throughput that was written to
    /// the TCP sockets.
    ///
    /// NOTE: Owing to truncation from nanosecond precision to seconds, the
    /// produced throughput may not be accurate for low write counts.
    pub fn throughput(&self) -> f64 {
        self.throughput
    }
}

#[cfg(test)]
mod test {
    use std::{net::TcpListener, str::FromStr, time::Instant};

    use humantime::Duration;

    use crate::{writer::WriteOptions, StreamWriter};

    #[tokio::test]
    async fn write() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();

        let input = b"hello";
        let size = input.len() as u64;
        let mut s = StreamWriter::new(
            listener.local_addr().unwrap(),
            input,
            WriteOptions::Count(1),
        );
        assert_eq!(s.write().await.unwrap(), size);

        let mut s = StreamWriter::new(
            listener.local_addr().unwrap(),
            input,
            WriteOptions::Count(5),
        );
        assert_eq!(
            s.write().await.unwrap(),
            size * 5,
            "Expected 5 times the input bytes"
        );
    }

    #[tokio::test]
    async fn write_for_duration() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();

        let input = b"duration_write";
        let duration = Duration::from_str("2s").unwrap();
        let mut s = StreamWriter::new(
            listener.local_addr().unwrap(),
            input,
            WriteOptions::Duration(duration),
        );

        let start = Instant::now();
        s.write().await.unwrap();
        let elapsed = start.elapsed().as_secs();
        assert_eq!(elapsed, 2);
    }

    #[tokio::test]
    async fn throughput() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();

        let mut s = StreamWriter::new(
            listener.local_addr().unwrap(),
            b"a",
            WriteOptions::Count(100),
        );
        s.write().await.unwrap();
        assert!(
            s.throughput() != 0.0,
            "Throughput should be set after writing data"
        );
    }
}
