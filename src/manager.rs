use std::net::{SocketAddr, ToSocketAddrs};

use futures::{stream::FuturesUnordered, StreamExt};
use tokio::{
    io::AsyncWriteExt,
    net::{TcpStream, UdpSocket},
    time::Instant,
};

use crate::{statistics::Statistics, Protocol};

/// Desired behaviour for how a socket should be written to.
#[derive(Debug)]
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
    /// Write a concurrent number of streams for a set duration.
    ConcurrencyWithDuration(u64, humantime::Duration),
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
            (Some(d), Some(c)) => WriteOptions::ConcurrencyWithDuration(c, d),
            (None, None) => WriteOptions::Count(count),
        }
    }
}

pub struct SocketManager<'a, S: ToSocketAddrs> {
    host: S,
    input: &'a [u8],
    protocol: Protocol,
    write_options: WriteOptions,
    stats: Statistics,
}

impl<'a, S> SocketManager<'a, S>
where
    S: ToSocketAddrs + Sync,
{
    /// Create a new [`SocketManager`]
    pub fn new(
        host: S,
        input: &'a [u8],
        protocol: Protocol,
        write_options: WriteOptions,
        stats: Statistics,
    ) -> Self {
        Self {
            host,
            input,
            write_options,
            protocol,
            stats,
        }
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
        for addr in addrs {
            match self.write_options {
                WriteOptions::Count(count) => {
                    for _ in 0..count {
                        match write_stream(addr, &self.protocol, self.input).await {
                            Ok(b) => {
                                self.stats.increment_total(b);
                                self.stats.record_success();
                            }
                            Err(_) => self.stats.record_failure(),
                        }
                    }
                }
                WriteOptions::Duration(duration) => {
                    let for_duration = Instant::now();
                    loop {
                        if for_duration.elapsed() >= *duration {
                            break;
                        } else {
                            match write_stream(addr, &self.protocol, self.input).await {
                                Ok(b) => {
                                    self.stats.increment_total(b);
                                    self.stats.record_success();
                                }
                                Err(_) => self.stats.record_failure(),
                            }
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
                            match write_stream(addr, &self.protocol, self.input).await {
                                Ok(b) => {
                                    self.stats.increment_total(b);
                                    self.stats.record_success();
                                }
                                Err(_) => self.stats.record_failure(),
                            }
                            sent += 1;
                        }
                    }
                }
                WriteOptions::ConcurrencyWithCount(concurrency, count) => {
                    let mut futs = FuturesUnordered::new();
                    let requests_per_task = count / concurrency;
                    for _ in 0..concurrency {
                        let input = self.input.to_owned();
                        let protocol = self.protocol.clone();
                        let task = tokio::spawn(async move {
                            let mut task_bytes = 0;
                            let mut success: u64 = 0;
                            let mut failure: u64 = 0;
                            for _ in 0..requests_per_task {
                                match write_stream(addr, &protocol, &input).await {
                                    Ok(b) => {
                                        task_bytes += b;
                                        success += 1;
                                    }
                                    Err(_) => failure += 1,
                                }
                            }
                            (task_bytes, success, failure)
                        });
                        futs.push(task);
                    }
                    while let Some(task) = futs.next().await {
                        let (written, success_count, failure_count) = task?;
                        self.stats.increment_total(written);
                        for _ in 0..success_count {
                            self.stats.record_success();
                        }
                        for _ in 0..failure_count {
                            self.stats.record_failure();
                        }
                    }
                }
                WriteOptions::ConcurrencyWithDuration(concurrency, duration) => {
                    let mut futs = FuturesUnordered::new();
                    for _ in 0..concurrency {
                        let input = self.input.to_owned();
                        let protocol = self.protocol.clone();
                        let task = tokio::spawn(async move {
                            let for_duration = Instant::now();
                            let mut task_bytes = 0;
                            let mut success: u64 = 0;
                            let mut failure: u64 = 0;
                            loop {
                                if for_duration.elapsed() >= *duration {
                                    break;
                                } else {
                                    match write_stream(addr, &protocol, &input).await {
                                        Ok(b) => {
                                            task_bytes += b;
                                            success += 1;
                                        }
                                        Err(_) => failure += 1,
                                    }
                                }
                            }
                            (task_bytes, success, failure)
                        });
                        futs.push(task);
                    }
                    while let Some(task) = futs.next().await {
                        let (written, success_count, failure_count) = task?;
                        self.stats.increment_total(written);
                        for _ in 0..success_count {
                            self.stats.record_success();
                        }
                        for _ in 0..failure_count {
                            self.stats.record_failure();
                        }
                    }
                }
            }
        }

        self.stats.record_throughput();
        Ok(self.stats.total_bytes())
    }

    /// Get the recorded throughput from the internal [`Statistics`].
    pub fn throughput(&self) -> f64 {
        self.stats.throughput()
    }

    /// Get the total number of bytes from the internal [`Statistics`].
    pub fn total_bytes(&self) -> u64 {
        self.stats.total_bytes()
    }

    /// The number of successful requests from the internal [`Statistics`].
    pub fn successful_requests(&self) -> u64 {
        self.stats.successful_requests()
    }

    /// Percentage of requests that were successful from the internal [`Statistics`].
    pub fn successful_requests_percentage(&self) -> f64 {
        self.stats.success_percentage()
    }

    pub fn elapsed(&self) -> u128 {
        self.stats.elapsed()
    }
}

/// Write the provided input data to a [`SocketAddr`] using the chosen [`Protocol`].
async fn write_stream(addr: SocketAddr, protocol: &Protocol, input: &[u8]) -> crate::Result<u64> {
    let out: u64;
    match protocol {
        Protocol::Tcp => {
            let mut stream = TcpStream::connect(addr).await?;
            stream.write_all(input).await?;
            out = input.len() as u64;
        }
        Protocol::Udp => {
            // Binding to 0 mimics the functionality of an unspecified socket.
            // It simply assigns a random port for the UDP socket to begin writing.
            // Ref: https://man7.org/linux/man-pages/man7/udp.7.html
            let stream = UdpSocket::bind("127.0.0.1:0").await?;
            out = stream.send_to(input, addr).await? as u64;
        }
    }
    Ok(out)
}

#[cfg(test)]
mod test {
    use std::{
        net::{SocketAddr, TcpListener},
        str::FromStr,
        time::Instant,
    };

    use humantime::Duration;

    use crate::{manager::WriteOptions, statistics::Statistics, Protocol, SocketManager};

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
        opts = WriteOptions::from_flags(1, None, None),
        expected = WriteOptions::Count(1)
    );
    write_options!(
        from_flags_non_default_count,
        opts = WriteOptions::from_flags(100_000_000, None, None),
        expected = WriteOptions::Count(100_000_000)
    );
    write_options!(
        from_flags_duration,
        opts =
            WriteOptions::from_flags(1, Some(humantime::Duration::from_str("10s").unwrap()), None),
        expected = WriteOptions::Duration(_)
    );
    write_options!(
        from_flags_count_or_duration,
        opts =
            WriteOptions::from_flags(3, Some(humantime::Duration::from_str("10s").unwrap()), None),
        expected = WriteOptions::CountOrDuration(3, _)
    );
    write_options!(
        from_flags_concurrency_count,
        opts = WriteOptions::from_flags(100, None, Some(10)),
        expected = WriteOptions::ConcurrencyWithCount(10, 100)
    );
    write_options!(
        from_flags_concurrency_duration,
        opts = WriteOptions::from_flags(
            1,
            Some(humantime::Duration::from_str("10s").unwrap()),
            Some(10)
        ),
        expected = WriteOptions::ConcurrencyWithDuration(10, _)
    );

    /// Encompass the count variant of the write options into a macro for ease of
    /// use of testing various scenarios
    macro_rules! write_count {
        ($name:ident, input = $input:expr, protocol = $protocol:expr, count = $count:expr, expected = $expected:expr) => {
            #[tokio::test]
            async fn $name() {
                let listener = TcpListener::bind("127.0.0.1:0").unwrap();
                let mut s = SocketManager::new(
                    listener.local_addr().unwrap(),
                    $input,
                    $protocol,
                    WriteOptions::Count($count),
                    Statistics::new(),
                );
                assert_eq!(s.write().await.unwrap(), $expected);
            }
        };
    }

    write_count!(
        write_single_tcp,
        input = b"hello",
        protocol = Protocol::Tcp,
        count = 1,
        expected = 5
    );
    write_count!(
        write_single_udp,
        input = b"hello",
        protocol = Protocol::Udp,
        count = 1,
        expected = 5
    );
    write_count!(
        write_multiple_tcp,
        input = b"hello",
        protocol = Protocol::Tcp,
        count = 5,
        expected = 25
    );
    write_count!(
        write_multiple_udp,
        input = b"hello",
        protocol = Protocol::Udp,
        count = 5,
        expected = 25
    );
    write_count!(
        write_large_tcp,
        input = b"wow-there's-a-lot-of-text-here",
        protocol = Protocol::Tcp,
        count = 3,
        expected = 90
    );
    write_count!(
        write_large_udp,
        input = b"wow-there's-a-lot-of-text-here",
        protocol = Protocol::Udp,
        count = 3,
        expected = 90
    );
    write_count!(
        write_tiny_tcp,
        input = b"a",
        protocol = Protocol::Tcp,
        count = 1,
        expected = 1
    );
    write_count!(
        write_tiny_udp,
        input = b"a",
        protocol = Protocol::Udp,
        count = 1,
        expected = 1
    );
    write_count!(
        write_tiny_multiple_tcp,
        input = b"a",
        protocol = Protocol::Tcp,
        count = 100,
        expected = 100
    );
    write_count!(
        write_tiny_multiple_udp,
        input = b"a",
        protocol = Protocol::Udp,
        count = 100,
        expected = 100
    );

    async fn bind_socket(protocol: &Protocol) -> SocketAddr {
        match protocol {
            Protocol::Tcp => {
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
                addr
            }
            Protocol::Udp => {
                let socket = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
                socket.local_addr().unwrap()
            }
        }
    }

    #[tokio::test]
    async fn write_for_duration() {
        let input = b"duration";
        let duration = Duration::from_str("2s").unwrap();
        let protocols = vec![Protocol::Tcp, Protocol::Udp];

        for protocol in protocols {
            let addr = bind_socket(&protocol).await;
            let mut s = SocketManager::new(
                addr,
                input,
                protocol.clone(),
                WriteOptions::Duration(duration),
                Statistics::default(),
            );
            let start = Instant::now();
            s.write().await.unwrap();
            let elapsed = start.elapsed().as_secs();
            assert_eq!(elapsed, 2);
            println!("[{protocol}] Wrote {} bytes per second", s.throughput());
        }
    }

    #[tokio::test]
    async fn write_concurrency() {
        let protocols = vec![Protocol::Tcp, Protocol::Udp];

        for protocol in protocols {
            let addr = bind_socket(&protocol).await;
            let input = b"c";
            let mut s = SocketManager::new(
                addr,
                input,
                protocol.clone(),
                WriteOptions::ConcurrencyWithCount(5, 100_000),
                Statistics::default(),
            );
            assert_eq!(s.write().await.unwrap(), 100_000);
            println!("[{protocol}] Wrote {} bytes per second", s.throughput());
        }
    }

    #[tokio::test]
    async fn write_concurrency_with_duration() {
        let protocols = vec![Protocol::Tcp, Protocol::Udp];
        let input = b"concurrent_duration";
        for protocol in protocols {
            let addr = bind_socket(&protocol).await;
            let duration = humantime::Duration::from_str("2s").unwrap();
            let mut s = SocketManager::new(
                addr,
                input,
                protocol.clone(),
                WriteOptions::ConcurrencyWithDuration(10, duration),
                Statistics::default(),
            );

            let start = Instant::now();
            s.write().await.unwrap();
            let elapsed = start.elapsed().as_secs();
            assert_eq!(elapsed, 2);
            assert!(s.throughput() > 0.0);
            assert!(
                s.stats.total_bytes() > input.len() as u64 * 100,
                "[{protocol}] More than 100 requests should be sent"
            );
            println!("[{protocol}] Wrote {} bytes per second", s.throughput());
        }
    }

    async fn throughput_helper(protocol: Protocol) {
        let addr = bind_socket(&protocol).await;
        let mut s = SocketManager::new(
            addr,
            b"a",
            protocol.clone(),
            WriteOptions::Count(100),
            Statistics::new(),
        );
        s.write().await.unwrap();
        assert!(
            s.throughput() != 0.0,
            "Throughput should be set after writing {protocol} data"
        );
    }

    #[tokio::test]
    async fn throughput() {
        throughput_helper(Protocol::Tcp).await;
        throughput_helper(Protocol::Udp).await;
    }
}
