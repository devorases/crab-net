# Crab Net

A CLI tool to generate TCP/TLS & UDP/DTLS traffic based on [Tokio framework](https://https://tokio.rs).

# Cargo Install

```
cargo install crab-net
```

# Multiple Payloads Support

The tool supports loading multiple payloads from a YAML file. Create a `payloads.yml` file with your payloads:

```yaml
payloads:
  - index: 0
    data: "test"
  - index: 1
    data: "hello world"
  - index: 2
    data: "this is a longer test payload"
  - index: 3
    data: "!@#$%^&*()"
```

A sample configuration file is provided as `payloads.yml.sample`. Copy it to create your own:

```bash
cp payloads.yml.sample payloads.yml
```

## Using Multiple Payloads

To use a specific payload by index:
```bash
./crab-net -d 127.0.0.1:8080 --payload-file payloads.yml --payload-index 2 --udp
```

To continuously alternate between payloads randomly:
```bash
./crab-net -d 127.0.0.1:8080 --payload-file payloads.yml --random-payload --udp
```

This will randomly select a different payload for each packet sent, creating more varied traffic patterns.

# Help

```
./crab-net --help
Simple stress test for servers

Usage: crab-net [OPTIONS] --destination <addr>

Options:
  -d, --destination <addr>      Server address as IP:PORT
  -c, --connections <clients>   Number of clients to simulate [default: 1]
  -r, --rate <rate>            Defined as packets/sec [default: 1]
  -p, --port <port>            Starting source port for clients [default: 8000]
  -l, --payload <payload>      Custom payload string to send [default: test]
      --payload-file <file>    YAML file containing multiple payloads
      --payload-index <index>  Use specific payload index from file
      --random-payload         Randomly select payload from file
  -w, --workers <workers>      Number of worker threads for the Tokio runtime [default: #CPU core]
  -s, --timeout <timeout>      Timeout between consecutive connections spawn as ms [default: 50]
      --udp                    Send packets via UDP
      --tls                    Send data over TLS
      --ca <ca>               PEM File to validate server credentials
  -h, --help                  Print help
  -V, --version               Print version
```
