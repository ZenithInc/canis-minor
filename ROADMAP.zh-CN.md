# 路线图

[English](ROADMAP.md)

本路线图描述方向，不构成交付承诺。优先级可能根据安全问题、运维证据和维护者精力调整。

## Now — 0.1.x

- 提升 Linux x86_64 稳定性与故障恢复能力。
- 让 Release 可复现，并持续提供可验证的校验和与构建来源证明。
- 完善运维、部署、升级、恢复和安全文档。
- 在真实 Ubuntu 24.04 TPROXY 环境中建立周期性回归验证。

## Next — 0.2

- 提供完整的 macOS 13+ 透明客户端。
- 支持 macOS 服务端和通用二进制。
- 使用 Swift 宿主程序与 `NETransparentProxyProvider` system extension，同时复用 Rust 协议核心。
- 实现与 Linux 对等的 TCP、UDP、DNS 改写、断线直连和自动重连行为。

macOS 可分发制品以 Apple Developer ID、Network Extension entitlement、代码签名和公证为前提。这些是 Apple 平台的硬性部署约束，不是可选的打包工作。参见 Apple 的 [`NETransparentProxyProvider` 文档](https://developer.apple.com/documentation/networkextension/netransparentproxyprovider)。

## Later

- Linux ARM64。
- IPv6。
- 多客户端身份与密钥轮换。
- 指标与结构化诊断。

## Not planned

- 中短期 Windows 支持。
- 保证截获容器网络命名空间流量。
- ICMP 隧道。

实现大型路线图事项前请先讨论，参见 [CONTRIBUTING.md](CONTRIBUTING.md)。
