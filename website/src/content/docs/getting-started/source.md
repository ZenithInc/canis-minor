---
title: Build from source
description: Build and test Canis Minor from source with the pinned Rust toolchain.
---

The supported source-build host is Ubuntu 24.04 x86_64. The repository pins Rust 1.85.0 and uses the 2024 edition.

## Install prerequisites

```bash
sudo apt-get update
sudo apt-get install -y build-essential file openssl pkg-config
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Review rustup's installer before running it if it is not already part of your trusted toolchain process.

## Clone and test

```bash
git clone https://github.com/ZenithInc/canis-minor.git
cd canis-minor
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
bash tests/target-parser.sh
```

The Rust suite contains 19 tests covering configuration constraints, PSK mismatch, ciphertext tampering, protocol versions, IPv4 address encoding, UDP boundaries, TCP half-close, concurrent streams, destination failures, tunnel interruption, rule generation, and recovery.

## Initialize private material

The server address is mandatory; there is no project-operated default:

```bash
bash scripts/init.sh --server-address 192.0.2.1:5260
```

Replace `192.0.2.1` with your server IPv4 address. This writes `.canis/psk`, `.canis/server.toml`, and `.canis/client.toml`. They are ignored by Git. `--force` replaces all three and rotates the PSK, so coordinate both endpoints before using it.

## Build

```bash
bash scripts/build-server.sh
bash scripts/build-static.sh
```

`build-static.sh` installs the `x86_64-unknown-linux-musl` target and builds both client and server release binaries. Confirm they are static PIE executables and runnable:

```bash
file target/x86_64-unknown-linux-musl/release/canis-client
file target/x86_64-unknown-linux-musl/release/canis-server
ldd target/x86_64-unknown-linux-musl/release/canis-client || true
target/x86_64-unknown-linux-musl/release/canis-client --help
target/x86_64-unknown-linux-musl/release/canis-server --help
```

Continue to [configuration](../../reference/configuration/) and [deployment](../../operations/deployment/).
