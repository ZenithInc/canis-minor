# Acceptance record (2026-08-16)

[简体中文](ACCEPTANCE.zh-CN.md)

Environment: an anonymized server represented by RFC 5737 IPv4 address `203.0.113.10`; an Ubuntu 24.04 x86_64 client host. An independent SSH session remained available throughout the acceptance run.

## Network and registry

- The client's observed public egress matched `203.0.113.10`.
- IPv4 HTTP and HTTPS requests completed successfully with status `200`.
- `registry-1.docker.io` did not resolve to the known incorrect address used by the test fixture.
- Docker Registry `/v2/` returned `401`, as expected for an unauthenticated request.
- The Docker authentication endpoint returned `200`.
- Both TCP and UDP destination port 53 entered the tunnel and were rewritten to the configured `1.1.1.1:53` upstream by the server.

## APT and Docker

- `apt-get update`: 38 seconds.
- Ubuntu `docker.io` and dependencies installation: 197 seconds.
- Docker `29.1.3-0ubuntu3~24.04.2` reached the `active` service state.
- `alpine:latest`: 24 seconds, successful on the first attempt.
- `ubuntu:24.04`: 78 seconds, successful on the first attempt.

The timings are retained as an anonymized operational baseline, not as performance guarantees.

## Failure recovery

- Within one second after the server stopped, the client removed its nftables table, policy rule, and both transparent listeners.
- Running `canis-client cleanup` twice succeeded, demonstrating idempotent cleanup.
- The client reconnected within four seconds after the server returned, and public egress again matched `203.0.113.10`.
- After reconnection there was one TCP and one UDP transparent listener, no stale listener, no panic, and systemd reported `NRestarts=0`.
- Final state: `canis-client.service=active` and `docker.service=active`.

The deployed client binary and the local static build had the same SHA-256 digest:

```text
d4e540553fe90162e93758bc98258520c2f38d175be910ec1b90ac4421e297bd
```

## Repeat the acceptance procedure

Keep an independent management session and provider console available. Validate public egress, HTTP/HTTPS, TCP and UDP DNS, APT, image pulls, tunnel loss, idempotent cleanup, bounded reconnection, and the absence of stale nftables, routing, or listener state. See the [online acceptance guide](https://zenithinc.github.io/canis-minor/project/acceptance/) for exact commands.
