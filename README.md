# Canis Minor

[![CI](https://github.com/ZenithInc/canis-minor/actions/workflows/ci.yml/badge.svg)](https://github.com/ZenithInc/canis-minor/actions/workflows/ci.yml)
[![Pages](https://github.com/ZenithInc/canis-minor/actions/workflows/pages.yml/badge.svg)](https://github.com/ZenithInc/canis-minor/actions/workflows/pages.yml)
[![Release](https://img.shields.io/github/v/release/ZenithInc/canis-minor)](https://github.com/ZenithInc/canis-minor/releases/latest)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)

[简体中文](README.zh-CN.md) · [Documentation](https://zenithinc.github.io/canis-minor/) · [Roadmap](ROADMAP.md) · [Security](SECURITY.md)

Canis Minor is an encrypted IPv4 transparent proxy tunnel for Ubuntu 24.04 on x86_64. A client on the target host establishes one outbound connection to the server, then relays host-originated TCP and UDP traffic through that tunnel. HTTP and HTTPS are forwarded as opaque bytes; Canis Minor does not decrypt TLS.

> [!IMPORTANT]
> Version 0.1 is intended for controlled deployments by experienced Linux operators. The client changes host nftables and policy-routing state and therefore requires root or equivalent network capabilities.

## Features

- Transparent host TCP and UDP forwarding through Linux TPROXY.
- `TCP → Noise NNpsk0 (32-byte PSK) → Yamux` encrypted multiplexing.
- One Yamux stream per TCP connection or UDP five-tuple, with a default limit of 256 concurrent streams.
- UDP session expiry after 60 seconds of inactivity and correct TCP half-close behavior.
- Server-side redirection of tunneled TCP/UDP port 53 traffic to the configured DNS upstream.
- Fail-open recovery: interception state is removed when the tunnel disconnects or the process exits, followed by bounded 1–30 second reconnect backoff.
- Reproducible, static PIE Linux x86_64 release artifacts with checksums and GitHub build provenance.

## Architecture

```text
Host process
    │ IPv4 TCP/UDP (output chain)
    ▼
nftables TPROXY ──► canis-client ──► Noise NNpsk0 ──► Yamux ──► canis-server
                         │                                      │
                         └── disconnect: remove interception    └── destination
                                                                or DNS upstream
```

The client installs a dedicated `inet canis_minor` nftables table, one fwmark rule, and one local policy route only after the PSK handshake succeeds and both transparent listeners bind. It removes those owned objects on SIGINT, SIGTERM, normal exit, or tunnel failure. The systemd unit also cleans stale state before start and after stop.

## Support matrix

| Platform | Client | Server | Status |
| --- | --- | --- | --- |
| Ubuntu 24.04, Linux x86_64, IPv4 | Supported | Supported | Current 0.1.x target |
| macOS 13+ | Planned | Planned | 0.2; requires Apple signing, notarization, and Network Extension entitlement |
| Linux ARM64 | Planned | Planned | Later |
| IPv6 | Not yet supported | Not yet supported | Later |
| Windows | Not planned | Not planned | No medium-term support planned |

Container-network-namespace forwarding and ICMP are not guaranteed. See [Traffic boundaries](#traffic-boundaries).

## Install

### Release binaries

Download the client or server archive for `x86_64-unknown-linux-musl` from [GitHub Releases](https://github.com/ZenithInc/canis-minor/releases). Each release includes example configuration, applicable systemd install/uninstall material, the license files, `SHA256SUMS`, and GitHub build provenance.

```bash
curl -LO https://github.com/ZenithInc/canis-minor/releases/download/v0.1.0/canis-client-v0.1.0-x86_64-unknown-linux-musl.tar.gz
curl -LO https://github.com/ZenithInc/canis-minor/releases/download/v0.1.0/SHA256SUMS
sha256sum --check --ignore-missing SHA256SUMS
gh attestation verify canis-client-v0.1.0-x86_64-unknown-linux-musl.tar.gz \
  --repo ZenithInc/canis-minor
```

Review the archive before running its scripts. The release does not include a package repository, container image, or crates.io package.

### Build from source

The supported build environment is Ubuntu 24.04 with Rust 1.85.0. Install `build-essential`, `pkg-config`, `openssl`, and Rust through rustup, then run:

```bash
git clone https://github.com/ZenithInc/canis-minor.git
cd canis-minor
bash scripts/init.sh --server-address 192.0.2.1:5260
bash scripts/build-server.sh
bash scripts/build-static.sh
```

Replace the RFC 5737 documentation address with your server's reachable IPv4 address. Initialization has no server default and refuses to run without `--server-address`. It creates ignored `.canis/psk`, `.canis/server.toml`, and `.canis/client.toml` files. The PSK is 32 raw random bytes with mode `0600`; never paste it into issues, logs, chat, or version control.

## Configure and run

The checked-in examples are [client.toml](examples/client.toml) and [server.toml](examples/server.toml). Important defaults are:

| Setting | Default | Purpose |
| --- | --- | --- |
| `listen` | `0.0.0.0:5260` | Server tunnel listener |
| `server_addr` | `192.0.2.1:5260` | Documentation-only client target; replace it |
| `dns_upstream` | `1.1.1.1:53` | Server destination for tunneled port 53 traffic |
| `proxy_port` | `15260` | Local transparent TCP/UDP listener |
| `max_streams` | `256` | Maximum multiplexed streams |
| `udp_idle_secs` | `60` | UDP association idle timeout |
| `reconnect_min_secs` / `reconnect_max_secs` | `1` / `30` | Client reconnect bounds |

Run the server in the foreground after allowing inbound TCP 5260:

```bash
bash scripts/run-server.sh
# RUST_LOG=debug bash scripts/run-server.sh
```

For source deployments, copy `server.txt.example` to the ignored `server.txt`, replace its target, configure SSH key authentication, and deploy:

```bash
cp server.txt.example server.txt
bash scripts/deploy-client.sh server.txt
ssh -p 22222 root@192.0.2.10 systemctl status canis-client
```

The target must provide `nft`, `ip`, systemd, Linux TPROXY support, and root/CAP_NET_ADMIN. SSH parsing accepts only one `ssh [-p PORT] user@IPv4` target, uses batch authentication, stores the first host key in ignored `.canis/known_hosts`, and rejects later key changes.

## Traffic boundaries

- Only host-originated IPv4 TCP/UDP on the nftables `output` path is intercepted.
- Traffic forwarded from container network namespaces through the `forward` path is outside the guarantee. Host processes such as `dockerd` and `containerd` are covered.
- Loopback, broadcast, multicast, link-local, DHCP, the tunnel server connection, and reply traffic for inbound connections bypass the proxy.
- ICMP, IPv6, and protocols other than TCP/UDP keep their existing system behavior.
- Any tunneled destination on TCP or UDP port 53 is rewritten by the server to `dns_upstream`.

## Security model

The tunnel provides encryption and possession-based authentication through a single 32-byte PSK. It does not provide per-client identities or automated key rotation. Anyone with the PSK can authenticate, and the server permits only one authenticated session at a time, replacing the previous session. Protect the PSK at rest, transport it over an authenticated channel, and rotate it manually after suspected disclosure.

The server is an outbound relay: a compromised authenticated client can ask it to connect to arbitrary reachable IPv4 TCP/UDP destinations. Restrict server network reachability and firewall rules accordingly. Canis Minor is not an anonymity service, a TLS inspection proxy, or a substitute for endpoint hardening. See the full [architecture and security documentation](https://zenithinc.github.io/canis-minor/concepts/architecture/).

Report vulnerabilities privately as described in [SECURITY.md](SECURITY.md). Do not open a public issue for an undisclosed vulnerability.

## Failure recovery and uninstall

Keep an independent SSH session and provider console available during first deployment. If the server becomes unavailable, the client removes `inet canis_minor`, its policy rule, and its local route before returning traffic to the host's normal direct path. Inspect cleanup with:

```bash
sudo nft list table inet canis_minor
ip -4 rule show
ip -4 route show table 5260
```

To uninstall a source deployment:

```bash
bash scripts/uninstall-client.sh server.txt
```

If remote automation is unavailable, run `canis-client cleanup --config /etc/canis-minor/client.toml` before removing the service or binary. Detailed recovery steps are in the [troubleshooting guide](https://zenithinc.github.io/canis-minor/operations/troubleshooting/).

## Development

```bash
make test     # 19 Rust tests plus the SSH target parser
make check    # rustfmt and Clippy with warnings denied
```

The real TPROXY acceptance procedure and anonymized Ubuntu 24.04 results are in [ACCEPTANCE.md](ACCEPTANCE.md). Website development lives in `website/` and uses Node 24 with pnpm.

Contributions are welcome. Read [CONTRIBUTING.md](CONTRIBUTING.md), the [Code of Conduct](CODE_OF_CONDUCT.md), [SUPPORT.md](SUPPORT.md), and the [roadmap](ROADMAP.md) before opening a pull request.

## License

Copyright 2026 ZenithInc. Licensed at your option under either the [MIT License](LICENSE-MIT) or the [Apache License 2.0](LICENSE-APACHE).
