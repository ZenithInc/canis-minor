---
title: Traffic boundaries
description: Know exactly which traffic Canis Minor 0.1 intercepts, bypasses, or leaves unsupported.
---

The current guarantee is deliberately narrow: host-originated IPv4 TCP and UDP on Ubuntu 24.04 x86_64.

## Intercepted

- IPv4 TCP connections created by normal host processes.
- IPv4 UDP datagrams created by normal host processes.
- Host-daemon traffic such as `dockerd` and `containerd` image pulls.
- TCP and UDP destination port 53, which enter the tunnel and are rewritten by the server to `dns_upstream`.

Interception occurs in the nftables `output` path. Original destination metadata is recovered at the transparent listeners and sent in a versioned request on a Yamux stream.

## Explicit bypasses

Rules bypass these categories to preserve host operation and prevent loops:

- Loopback and local destinations.
- IPv4 broadcast, multicast, and link-local ranges.
- DHCP traffic.
- The configured tunnel server connection.
- Replies belonging to inbound connections.
- Canis Minor sockets carrying the configured bypass mark.

The exact generated rules are covered by unit tests, but operators should inspect them on their kernel and nftables version during acceptance.

## Outside the guarantee

| Traffic                                                | 0.1 behavior                                                          |
| ------------------------------------------------------ | --------------------------------------------------------------------- |
| Container namespace traffic forwarded through the host | Not guaranteed; it commonly traverses `forward`, not host `output`.   |
| ICMP                                                   | Left to the host's existing network path.                             |
| IPv6                                                   | Not intercepted or tunneled.                                          |
| Protocols other than TCP/UDP                           | Left unchanged.                                                       |
| Inbound connections to the client host                 | Not proxied; their replies are deliberately bypassed.                 |
| Multiple simultaneous clients                          | Not supported; a new authenticated session replaces the previous one. |

## DNS consequences

All tunneled destinations whose port is 53 are rewritten, regardless of the original IPv4 address. This covers classic UDP DNS and TCP fallback, but not DNS-over-HTTPS, DNS-over-TLS on other ports, application-specific encrypted DNS, or IPv6 DNS traffic.

Changing `dns_upstream` changes where those tunneled queries are sent. It does not configure `/etc/resolv.conf`, systemd-resolved, browser secure-DNS settings, or container namespace DNS.

## Design your policy around the boundary

Canis Minor is not a comprehensive host firewall. If organizational policy requires blocking direct egress, IPv6, private-network reachability, encrypted DNS, or container traffic, enforce those requirements independently. Validate both tunnel-up and tunnel-down states because fail-open recovery intentionally restores the original direct path.
