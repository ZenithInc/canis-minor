---
title: Configuration reference
description: Complete client and server TOML configuration reference for Canis Minor 0.1.
---

Canis Minor reads strict TOML and rejects unknown fields. IPv4 socket addresses use quoted `IPv4:PORT` form. Duration values are whole seconds.

## Client

```toml
server_addr = "192.0.2.1:5260"
psk_file = "/etc/canis-minor/psk"
proxy_port = 15260
fwmark = 13259360
bypass_mark = 13259361
route_table = 5260
rule_priority = 10526
nft_table = "canis_minor"
handshake_timeout_secs = 10
heartbeat_secs = 5
udp_idle_secs = 60
reconnect_min_secs = 1
reconnect_max_secs = 30
max_streams = 256
```

| Field                    | Required/default | Constraints and behavior                                                                              |
| ------------------------ | ---------------- | ----------------------------------------------------------------------------------------------------- |
| `server_addr`            | Required         | Concrete IPv4 address and nonzero TCP port. `0.0.0.0` is rejected.                                    |
| `psk_file`               | Required         | Path to exactly 32 raw bytes. On Unix, group/other permission bits must be clear.                     |
| `proxy_port`             | `15260`          | Nonzero local TCP and UDP transparent-listener port.                                                  |
| `fwmark`                 | `13259360`       | Nonzero mark applied to intercepted traffic for policy routing.                                       |
| `bypass_mark`            | `13259361`       | Nonzero, different mark applied to Canis Minor's own outbound sockets to avoid recursion.             |
| `route_table`            | `5260`           | Positive policy-routing table number.                                                                 |
| `rule_priority`          | `10526`          | Positive IPv4 rule priority.                                                                          |
| `nft_table`              | `canis_minor`    | Nonempty ASCII letters, digits, and underscores only. The whole `inet` table is owned by Canis Minor. |
| `handshake_timeout_secs` | `10`             | Positive Noise handshake deadline.                                                                    |
| `heartbeat_secs`         | `5`              | Positive interval used to drive Yamux health/keepalive activity.                                      |
| `udp_idle_secs`          | `60`             | Positive client UDP five-tuple idle lifetime.                                                         |
| `reconnect_min_secs`     | `1`              | Positive initial retry delay.                                                                         |
| `reconnect_max_secs`     | `30`             | Must be at least the minimum; exponential backoff is capped here.                                     |
| `max_streams`            | `256`            | From 1 through 256 inclusive. Also bounds local concurrent proxy work.                                |

Do not reuse `proxy_port`, marks, route table, rule priority, or nft table name for unrelated host networking. If multiple installations share a host, assign unique values and test cleanup carefully.

## Server

```toml
listen = "0.0.0.0:5260"
psk_file = ".canis/psk"
dns_upstream = "1.1.1.1:53"
handshake_timeout_secs = 10
connect_timeout_secs = 10
udp_idle_secs = 60
heartbeat_secs = 5
max_streams = 256
```

| Field                    | Required/default | Constraints and behavior                                                                                  |
| ------------------------ | ---------------- | --------------------------------------------------------------------------------------------------------- |
| `listen`                 | Required         | IPv4 bind address with a nonzero TCP port.                                                                |
| `psk_file`               | Required         | Same 32-byte raw PSK as the client, with private permissions.                                             |
| `dns_upstream`           | `1.1.1.1:53`     | IPv4 upstream; the port must be exactly 53. Every tunneled TCP/UDP destination port 53 is rewritten here. |
| `handshake_timeout_secs` | `10`             | Positive Noise handshake deadline.                                                                        |
| `connect_timeout_secs`   | `10`             | Positive deadline for server-side TCP destination connects.                                               |
| `udp_idle_secs`          | `60`             | Positive server UDP relay idle lifetime.                                                                  |
| `heartbeat_secs`         | `5`              | Positive Yamux drive interval.                                                                            |
| `max_streams`            | `256`            | From 1 through 256 inclusive.                                                                             |

The server accepts up to 64 pending handshakes. Only one authenticated tunnel session remains active: a newly authenticated session cancels the previous one.

## PSK lifecycle

Generate only with a cryptographically secure source:

```bash
umask 077
openssl rand 32 > psk
chmod 0600 psk
test "$(wc -c < psk)" -eq 32
```

The file is binary, not hexadecimal or base64 text. Distribute it over an authenticated confidential channel. To rotate it, stop the client, update both endpoints, restart the server, then start the client and repeat the acceptance checks. There is no overlap or multi-key grace period in 0.1.

## Logging

Logging uses `RUST_LOG`; the default level is `info`.

```bash
RUST_LOG=debug canis-server --config /etc/canis-minor/server.toml
RUST_LOG=canis_client=debug canis-client run --config /etc/canis-minor/client.toml
```

Treat logs as operationally sensitive even though the PSK is never intentionally logged. Redact peer and destination addresses before sharing.
