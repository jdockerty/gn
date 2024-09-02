# gn

A TCP/UDP tool like `nc`, but way dumber.

## Install

Install via `cargo`:

```sh
cargo install --git https://github.com/jdockerty/gn
```

## Usage

Write data using the `write` subcommand. This exposes numerous ways to interact
with the remote server, of which a new stream is used per write.

For example:

```sh
# Send 100 requests over TCP which contains "hello-world"
gn write --host 127.0.0.1:5000 --count 100 "hello-world"

# Write for a duration of 3s, from stdin, with "input"
echo "input" | gn write --host 127.0.0.1:5000 --duration 3s

# Write 5 concurrent requests for 1s over UDP
gn write --host 127.0.0.1:5000 --protocol udp --concurrency 5 "some_data"
```

This also bundles a server implementation using the `serve` subcommand, the
specified protocol to listen for incoming connections is determined via the
`--protocol` flag.

```
# Default to TCP
gn serve

# Listen for incoming UDP
gn serve --protocol udp
```

