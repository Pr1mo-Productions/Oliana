# Tests

This document tracks some one-liners and setups that can be used to test the underlying systems.

# Server-Client Connections

SSH to `stitch` (forwarding port 8011, happens by default with `cloud/remote-to-stitch.py`), move to `/home/user/Oliana` and start a server.

```bash
PORT=8011 cargo run --release --bin oliana_server
```

On your local machine, move do your `Oliana` directory and run

```bash
cargo build --release

# Replace `swayimg` w/ file viewer of choice
time ./target/release/oliana_client image --server-url '127.0.0.1:8011' -p "A skinny cow jumps over a green ocean wave" && swayimg out.png

# W/o the --output argument this streams text to stdout
time ./target/release/oliana_client text --server-url '127.0.0.1:8011' --system-prompt "You are a snappy flight attendant who tells terrible pun jokes." -p "Hello, my flight's been canceled, can you help me book another?"


```



