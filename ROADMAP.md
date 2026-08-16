# Roadmap

[简体中文](ROADMAP.zh-CN.md)

This roadmap describes intent, not a delivery commitment. Priorities may change based on security findings, operational evidence, and maintainer capacity.

## Now — 0.1.x

- Improve Linux x86_64 stability and failure recovery.
- Make releases reproducible and keep checksums and build provenance verifiable.
- Expand operations, deployment, upgrade, recovery, and security documentation.
- Add recurring regression coverage in a real Ubuntu 24.04 TPROXY environment.

## Next — 0.2

- Deliver a complete transparent client for macOS 13+.
- Support a macOS server and universal binaries.
- Use a Swift host application with an `NETransparentProxyProvider` system extension while reusing the Rust protocol core.
- Match Linux behavior for TCP, UDP, DNS redirection, fail-open direct connectivity, and automatic reconnection.

Distributable macOS artifacts depend on an Apple Developer ID, the Network Extension entitlement, code signing, and notarization. These are hard Apple platform deployment constraints, not optional packaging work. See Apple's [`NETransparentProxyProvider` documentation](https://developer.apple.com/documentation/networkextension/netransparentproxyprovider).

## Later

- Linux ARM64.
- IPv6.
- Multiple client identities and key rotation.
- Metrics and structured diagnostics.

## Not planned

- Windows support in the short or medium term.
- Guaranteed interception of container-network-namespace traffic.
- ICMP tunneling.

Please discuss large roadmap proposals before implementing them. See [CONTRIBUTING.md](CONTRIBUTING.md).
