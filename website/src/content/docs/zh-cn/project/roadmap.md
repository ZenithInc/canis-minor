---
title: 路线图
description: Canis Minor 的稳定化、macOS 工作、后续平台与明确非目标。
---

路线图表达方向，不构成交付承诺；安全问题与真实运维证据优先。

## Now — 0.1.x

- Linux x86_64 稳定性与故障恢复。
- 具有可验证校验和与来源证明的可复现 Release。
- 运维与安全文档。
- 在真实 Ubuntu 24.04 TPROXY 环境中周期回归。

## Next — 0.2

- 完整的 macOS 13+ 透明客户端。
- macOS 服务端与通用二进制。
- 使用 Swift 宿主应用和 `NETransparentProxyProvider` system extension，并复用 Rust 协议核心。
- TCP、UDP、DNS 改写、断线直连和自动重连行为与 Linux 对等。

macOS 可分发制品需要 Apple Developer ID、Network Extension entitlement、代码签名与公证。这些是 Apple 的硬性部署约束。参见 Apple 的 [`NETransparentProxyProvider` 文档](https://developer.apple.com/documentation/networkextension/netransparentproxyprovider)。

## Later

- Linux ARM64。
- IPv6。
- 多客户端身份与密钥轮换。
- 指标与结构化诊断。

## Not planned

- 中短期 Windows 支持。
- 保证容器网络命名空间流量。
- ICMP 隧道。

实现大型路线图事项前请先在 GitHub Discussions 讨论。仓库内规范副本见 [ROADMAP.zh-CN.md](https://github.com/ZenithInc/canis-minor/blob/main/ROADMAP.zh-CN.md)。
