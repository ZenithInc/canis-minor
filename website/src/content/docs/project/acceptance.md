---
title: Acceptance testing
description: Validate real Ubuntu TPROXY behavior, DNS, package traffic, recovery, and artifact identity.
---

Unit tests cannot validate a live kernel's TPROXY, nftables, policy routing, firewall, or provider network. Complete this procedure on every new deployment and after relevant upgrades.

:::danger
Keep an independent SSH session and provider console open. Record the client's original public IPv4 address and route state before enabling Canis Minor.
:::

## Baseline and active state

Before starting the client, capture:

```bash
curl -4 --fail --silent https://api.ipify.org; echo
ip -4 rule show
ip -4 route show
```

After starting it, confirm the service, table, rule, route, and exactly one TCP plus one UDP transparent listener:

```bash
systemctl is-active canis-client
sudo nft list table inet canis_minor
ip -4 rule show
ip -4 route show table 5260
ss -lntup | grep 15260
```

## Egress, web, and DNS

Replace `203.0.113.10` below with the server's actual public egress and compare it to the output:

```bash
test "$(curl -4 --fail --silent https://api.ipify.org)" = "203.0.113.10"
curl -4 --fail --silent --output /dev/null https://example.com/
getent ahostsv4 registry-1.docker.io
curl -4 -sS -o /dev/null -w '%{http_code}\n' https://registry-1.docker.io/v2/
curl -4 -sS -o /dev/null -w '%{http_code}\n' \
  'https://auth.docker.io/token?service=registry.docker.io&scope=repository:library/alpine:pull'
```

The Registry status should be `401` and the authentication status `200`. Capture server-side evidence that both UDP and TCP destination port 53 are reaching the configured upstream.

## APT and container pulls

```bash
time sudo apt-get update
time sudo apt-get install -y docker.io
sudo systemctl enable --now docker
time sudo docker pull alpine:latest
time sudo docker pull ubuntu:24.04
```

These cover host processes (`apt`, `dockerd`, and `containerd`), not arbitrary traffic inside a container network namespace.

## Fail-open and reconnect

1. Stop or firewall the server tunnel listener.
2. Within the configured failure-detection window, confirm `inet canis_minor`, its policy rule/route, and transparent listeners disappear.
3. Confirm the independent SSH session remains usable and direct IPv4 connectivity returns.
4. Run `canis-client cleanup` twice and confirm both invocations succeed.
5. Restore the server.
6. Confirm bounded automatic reconnection, one TCP and one UDP listener, no stale rules, no panic, and correct proxied egress.

## Record evidence

Record versions, redacted topology, results, timings, failure/recovery duration, `NRestarts`, and SHA-256 equivalence between the deployed binary and the verified build. Never include the PSK or live infrastructure identifiers.

The project's initial anonymized record is available in [ACCEPTANCE.md](https://github.com/ZenithInc/canis-minor/blob/main/ACCEPTANCE.md).
