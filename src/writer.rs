use std::net::{SocketAddr, ToSocketAddrs};

use tokio::{io::AsyncWriteExt, net::TcpStream, time::Instant};

/// Desired behaviour for how a socket should be written to.
pub enum WriteOptions {
    /// Write a `u64` number of streams.
    Count(u64),
    /// Write for a `Duration` length of time.
    Duration(humantime::Duration),
    /// Write a `u64` number of streams or write for a `Duration` length of time,
    /// whichever comes first.
    CountOrDuration(u64, humantime::Duration),
    /// Write a concurrent number of streams up to a particular count.
    ConcurrencyWithCount(u64, u64),
}

impl WriteOptions {
    /// Create [`WriteOptions`] from the known flags of the application which
    /// influence the behaviour of writes.
    pub fn from_flags(
        count: u64,
        duration: Option<humantime::Duration>,
        concurrency: Option<u64>,
    ) -> Self {
        match (duration, concurrency) {
            (Some(d), None) if count > 1 => WriteOptions::CountOrDuration(count, d),
            (Some(d), None) => WriteOptions::Duration(d),
            (None, Some(c)) => WriteOptions::ConcurrencyWithCount(c, count),
            (Some(_d), Some(_c)) => todo!(),
            (None, None) => WriteOptions::Count(count),
        }
    }
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
                WriteOptions::CountOrDuration(count, duration) => {
                    let for_duration = Instant::now();
                    let mut sent = 0;
                    loop {
                        if sent == count || for_duration.elapsed() >= *duration {
                            break;
                        } else {
                            self.write_stream(addr).await?;
                            sent += 1;
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

    macro_rules! write_options {
        ($name:ident, opts = $opts:expr, expected = $expected:pat) => {
            #[test]
            fn $name() {
                assert!(matches!($opts, $expected));
            }
        };
    }

    write_options!(
        from_flags_default_count,
        opts = WriteOptions::from_flags(1, None),
        expected = WriteOptions::Count(1)
    );
    write_options!(
        from_flags_non_default_count,
        opts = WriteOptions::from_flags(100_000_000, None),
        expected = WriteOptions::Count(100_000_000)
    );
    write_options!(
        from_flags_duration,
        opts = WriteOptions::from_flags(1, Some(humantime::Duration::from_str("10s").unwrap())),
        expected = WriteOptions::Duration(_)
    );
    write_options!(
        from_flags_count_or_duration,
        opts = WriteOptions::from_flags(3, Some(humantime::Duration::from_str("10s").unwrap())),
        expected = WriteOptions::CountOrDuration(3, _)
    );

    /// Encompass the count variant of the write options into a macro for ease of
    /// use of testing various scenarios
    macro_rules! write_count {
        ($name:ident, input = $input:expr, count = $count:expr, expected = $expected:expr) => {
            #[tokio::test]
            async fn $name() {
                let listener = TcpListener::bind("127.0.0.1:0").unwrap();
                let mut s = StreamWriter::new(
                    listener.local_addr().unwrap(),
                    $input,
                    WriteOptions::Count($count),
                );
                assert_eq!(s.write().await.unwrap(), $expected);
            }
        };
    }

    write_count!(write_single, input = b"hello", count = 1, expected = 5);
    write_count!(write_multiple, input = b"hello", count = 5, expected = 25);
    write_count!(
        write_large,
        input = b"wow-there's-a-lot-of-text-here",
        count = 3,
        expected = 90
    );
    write_count!(write_tiny, input = b"a", count = 1, expected = 1);
    write_count!(
        write_tiny_multiple,
        input = b"a",
        count = 100,
        expected = 100
    );

    #[tokio::test]
    async fn write_for_duration() {
        // Use a tokio listener to not block the runtime so that we can accept
        // the incoming connections. When writing for a duration the backlog of
        // the listen syscall can fill up, so we must accept the incoming connections,
        // even if they are discarded, otherwise the test can come to a halt.
        // See backlog parameter from https://man7.org/linux/man-pages/man2/listen.2.html
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            loop {
                listener.accept().await.unwrap();
            }
        });

        let input = b"duration";
        let duration = Duration::from_str("2s").unwrap();
        let mut s = StreamWriter::new(addr, input, WriteOptions::Duration(duration));

        let start = Instant::now();
        s.write().await.unwrap();
        let elapsed = start.elapsed().as_secs();
        assert_eq!(elapsed, 2);
        println!("Wrote {} bytes per second", s.throughput());
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
