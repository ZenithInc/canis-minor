---
title: Troubleshooting and recovery
description: Diagnose tunnel, TPROXY, DNS, routing, and fail-open recovery problems.
---

Start from safety: preserve an independent management connection and provider console. Avoid flushing all nftables or routing state on a shared host.

## Collect a redacted snapshot

```bash
systemctl status canis-client --no-pager
journalctl -u canis-client --since=-15m --no-pager
sudo nft list table inet canis_minor
ip -4 rule show
ip -4 route show table 5260
ss -lntup | grep 15260
```

Remove PSKs, usernames, SSH targets, host keys, and real public/private infrastructure addresses before sharing output.

## Client never activates interception

The client intentionally installs rules only after all of these succeed:

1. Direct TCP connection to `server_addr` using the bypass mark.
2. Noise handshake with the matching PSK.
3. Transparent TCP listener bind.
4. Transparent UDP listener bind.
5. Yamux startup.

Check server reachability, firewall state, clock-independent PSK equality, file length/permissions, port conflicts, root privileges, and required kernel modules. A wrong PSK appears as handshake failure rather than partial activation.

## PSK errors

```bash
wc -c /etc/canis-minor/psk
stat -c '%a %U:%G %n' /etc/canis-minor/psk
sha256sum /etc/canis-minor/psk  # compare only through a trusted admin channel
```

The key must be 32 raw bytes, not 64 hexadecimal characters or base64 text, and must not be group/other accessible. Avoid publishing even its digest because it can correlate deployments.

## Rules remain after a crash

The systemd `ExecStartPre` and `ExecStopPost` hooks call idempotent cleanup. Run it manually if needed:

```bash
sudo /usr/local/bin/canis-client cleanup \
  --config /etc/canis-minor/client.toml
```

Then inspect the configured table and route objects. Do not delete broad system rules. If the configuration was removed before cleanup, restore the exact identifiers in a temporary configuration or carefully remove only the known Canis Minor objects through the provider console.

## Traffic loops or no connectivity

Confirm `fwmark` and `bypass_mark` differ, the route table and priority do not collide with other policy systems, and `server_addr` is the actual concrete IPv4 endpoint. Verify that the server connection and inbound reply traffic match bypass rules. Conflicts with VPNs, other transparent proxies, or host-management agents require allocating unique marks/table/priority and retesting.

## DNS behaves unexpectedly

Canis Minor rewrites only tunneled TCP/UDP destination port 53. Check:

```bash
getent ahostsv4 registry-1.docker.io
dig +tcp @1.1.1.1 example.com
dig @1.1.1.1 example.com
```

Browser secure DNS, DNS-over-TLS, IPv6, and container namespace resolvers can follow different paths. The configured server `dns_upstream` must use port 53 and be reachable from the server.

## Tunnel repeatedly reconnects

Inspect both endpoint logs for handshake, encryption, Yamux, heartbeat, stream-limit, and destination errors. The delay doubles from `reconnect_min_secs` to `reconnect_max_secs`; sessions lasting at least 30 seconds reset it to the minimum. Check middlebox TCP idle timeouts and server replacement sessions—a second authenticated client intentionally disconnects the first.

## Emergency recovery

1. Use the independent console or SSH session.
2. `systemctl stop canis-client`.
3. Run the explicit cleanup command above.
4. Confirm the configured nft table, policy rule, and route are absent.
5. Test direct IPv4 connectivity.
6. Keep the service disabled until configuration and conflicts are understood.

If the failure may be a vulnerability, use [private reporting](https://github.com/ZenithInc/canis-minor/security/advisories/new) rather than a public issue.
