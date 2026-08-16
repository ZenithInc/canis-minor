---
title: Roadmap
description: Planned Canis Minor stabilization, macOS work, later platforms, and explicit non-goals.
---

The roadmap expresses intent, not a delivery commitment. Security findings and real operational evidence take priority.

## Now — 0.1.x

- Linux x86_64 stability and failure recovery.
- Reproducible releases with verifiable checksums and provenance.
- Operations and security documentation.
- Recurring regression in a real Ubuntu 24.04 TPROXY environment.

## Next — 0.2

- A complete transparent client for macOS 13+.
- A macOS server and universal binaries.
- A Swift host application and `NETransparentProxyProvider` system extension that reuse the Rust protocol core.
- Behavior equivalent to Linux for TCP, UDP, DNS redirection, fail-open direct connectivity, and automatic reconnection.

Distributable macOS artifacts require an Apple Developer ID, Network Extension entitlement, code signing, and notarization. These are hard Apple deployment constraints. See Apple's [`NETransparentProxyProvider` documentation](https://developer.apple.com/documentation/networkextension/netransparentproxyprovider).

## Later

- Linux ARM64.
- IPv6.
- Multiple client identities and key rotation.
- Metrics and structured diagnostics.

## Not planned

- Windows support in the short or medium term.
- Guaranteed container-network-namespace traffic.
- ICMP tunneling.

Discuss substantial roadmap proposals in GitHub Discussions before implementation. The canonical repository copy is [ROADMAP.md](https://github.com/ZenithInc/canis-minor/blob/main/ROADMAP.md).
