---
title: Architecture and security model
description: Understand Canis Minor's data plane, trust model, encryption, and privilege boundaries.
---

Canis Minor has three Rust crates: `canis-client`, `canis-server`, and a shared `canis-protocol` library. The client is the only component that changes host network state.

## Session lifecycle

1. The client removes stale objects bearing its configured nft table, marks, route table, and priority.
2. A bypass-marked TCP socket connects directly to `server_addr`.
3. Client and server complete a Noise NNpsk0 handshake using the same 32-byte PSK.
4. The client binds transparent TCP and UDP listeners.
5. A Yamux session starts over the encrypted transport.
6. Only then does the client install its nftables table, policy rule, and local route.
7. Each intercepted TCP connection or UDP five-tuple opens one multiplexed stream carrying a versioned destination request.
8. Any listener, multiplexing, encryption, tunnel, or shutdown event cancels proxy work and removes interception state before reconnection.

This ordering prevents the host from installing a black-hole route before an authenticated usable tunnel exists.

## Data plane

```text
application TCP/UDP
        │
        ▼
inet canis_minor / output
        │ mark + TPROXY route
        ▼
transparent listener
        │ destination metadata + payload
        ▼
Noise transport ─ Yamux stream ─ server socket ─ destination
```

TCP streams preserve bidirectional half-close. UDP streams preserve datagram boundaries, including empty datagrams. Both sides enforce `max_streams`; excess work is rejected rather than queued without bound.

## Cryptographic boundary

Noise provides encrypted transport and proves possession of the shared PSK. TLS carried inside the tunnel remains end to end between the originating application and its destination; Canis Minor neither terminates nor inspects it.

Version 0.1 has no certificate PKI, named client identity, multi-client authorization, or automatic rotation. The PSK is therefore a high-value bearer secret. A person who obtains it can impersonate an endpoint. The protocol should not be exposed with a weak, reused, text-encoded, or publicly distributed key.

## Server trust and reachability

An authenticated client can request outbound TCP or UDP connections to IPv4 destinations reachable by the server. This includes private networks unless the operating system or firewall blocks them. Treat the server as a privileged network relay:

- Run it on a dedicated or appropriately isolated host.
- Constrain ingress to the configured TCP tunnel port.
- Constrain egress to destinations appropriate for your deployment.
- Protect and monitor its configuration and PSK.
- Do not claim anonymity: destinations observe the server's egress address, and operators can observe metadata.

The server permits one active authenticated session. A new successful handshake supersedes the old session; this is recovery behavior, not multi-client support.

## Client privilege boundary

Transparent interception requires root or Linux network capabilities. The provided systemd unit limits address families and capabilities, uses `NoNewPrivileges`, protects system paths, and denies writable executable memory, but it still runs as root because it must manage nftables and routes.

The cleanup command constructs only the configured owned object names and is designed to be idempotent. Operators must avoid choosing identifiers used by other policy routing or nftables systems.

## Failure model

Canis Minor chooses direct connectivity over persistent interception when its secure path fails. A tunnel outage may therefore change the observed public egress and bypass controls that existed only on the relay. If direct traffic must be blocked, enforce that separately in host or network policy; do not rely on Canis Minor's fail-open behavior as a kill switch.

Read [traffic boundaries](../traffic-boundaries/) before deploying and [Security Policy](https://github.com/ZenithInc/canis-minor/blob/main/SECURITY.md) before reporting a vulnerability.
