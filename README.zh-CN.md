# Canis Minor

[![CI](https://github.com/ZenithInc/canis-minor/actions/workflows/ci.yml/badge.svg)](https://github.com/ZenithInc/canis-minor/actions/workflows/ci.yml)
[![Pages](https://github.com/ZenithInc/canis-minor/actions/workflows/pages.yml/badge.svg)](https://github.com/ZenithInc/canis-minor/actions/workflows/pages.yml)
[![Release](https://img.shields.io/github/v/release/ZenithInc/canis-minor)](https://github.com/ZenithInc/canis-minor/releases/latest)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)

[English](README.md) · [中文文档](https://zenithinc.github.io/canis-minor/zh-cn/) · [中文路线图](ROADMAP.zh-CN.md) · [安全策略](SECURITY.md)

Canis Minor 是面向 Ubuntu 24.04 x86_64 的 IPv4 加密透明代理隧道。目标机客户端主动建立一条到服务端的出站连接，再通过隧道转发主机进程发起的 TCP 和 UDP 流量。HTTP 与 HTTPS 都按不透明字节转发；Canis Minor 不解密 TLS。

> [!IMPORTANT]
> 0.1 版本面向有 Linux 运维经验的受控部署。客户端会修改主机 nftables 和策略路由状态，因此需要 root 或等效网络权限。

## 特性

- 基于 Linux TPROXY 的主机 TCP/UDP 透明转发。
- `TCP → Noise NNpsk0（32 字节 PSK）→ Yamux` 加密多路复用。
- 每个 TCP 连接或 UDP 五元组使用一个 Yamux 子流，默认最多 256 个并发子流。
- UDP 会话空闲 60 秒后回收，TCP 保留双向半关闭语义。
- 服务端将隧道内所有 TCP/UDP 53 端口流量改写到配置的 DNS 上游。
- 断线直连恢复：隧道断开或进程退出时移除截获状态，并按 1–30 秒有上限退避重连。
- 提供可复现的 Linux x86_64 静态 PIE 制品、校验和与 GitHub 构建来源证明。

## 架构

```text
主机进程
    │ IPv4 TCP/UDP（output 链）
    ▼
nftables TPROXY ──► canis-client ──► Noise NNpsk0 ──► Yamux ──► canis-server
                         │                                      │
                         └── 断线时移除截获状态                 └── 原始目标
                                                                或 DNS 上游
```

只有在 PSK 握手成功且 TCP/UDP 透明监听器均绑定成功后，客户端才会安装独立的 `inet canis_minor` nftables 表、一条 fwmark 规则和一条本地策略路由。收到 SIGINT/SIGTERM、正常退出或隧道失败时，只删除自己拥有的对象。systemd 单元还会在启动前和停止后清理遗留状态。

## 支持矩阵

| 平台 | 客户端 | 服务端 | 状态 |
| --- | --- | --- | --- |
| Ubuntu 24.04、Linux x86_64、IPv4 | 支持 | 支持 | 当前 0.1.x 目标 |
| macOS 13+ | 计划中 | 计划中 | 0.2；依赖 Apple 签名、公证和 Network Extension entitlement |
| Linux ARM64 | 计划中 | 计划中 | Later |
| IPv6 | 尚不支持 | 尚不支持 | Later |
| Windows | 不计划 | 不计划 | 中短期无支持计划 |

容器网络命名空间转发和 ICMP 不在当前保证范围，详见[流量边界](#流量边界)。

## 安装

### Release 二进制

从 [GitHub Releases](https://github.com/ZenithInc/canis-minor/releases) 下载 `x86_64-unknown-linux-musl` 客户端或服务端归档。每个 Release 都包含配置示例、对应的 systemd 安装/卸载材料、许可文件、`SHA256SUMS` 和 GitHub 构建来源证明。

```bash
curl -LO https://github.com/ZenithInc/canis-minor/releases/download/v0.1.0/canis-client-v0.1.0-x86_64-unknown-linux-musl.tar.gz
curl -LO https://github.com/ZenithInc/canis-minor/releases/download/v0.1.0/SHA256SUMS
sha256sum --check --ignore-missing SHA256SUMS
gh attestation verify canis-client-v0.1.0-x86_64-unknown-linux-musl.tar.gz \
  --repo ZenithInc/canis-minor
```

执行脚本前请先检查归档内容。项目暂不提供系统软件源、容器镜像或 crates.io 包。

### 从源码构建

正式支持的构建环境是 Ubuntu 24.04 与 Rust 1.85.0。安装 `build-essential`、`pkg-config`、`openssl` 和 rustup 后执行：

```bash
git clone https://github.com/ZenithInc/canis-minor.git
cd canis-minor
bash scripts/init.sh --server-address 192.0.2.1:5260
bash scripts/build-server.sh
bash scripts/build-static.sh
```

请将 RFC 5737 文档地址替换为服务端可达的 IPv4 地址。初始化脚本没有默认服务器，缺少 `--server-address` 时会拒绝运行。它会生成已忽略的 `.canis/psk`、`.canis/server.toml` 和 `.canis/client.toml`。PSK 是权限为 `0600` 的 32 个原始随机字节；不要将它粘贴到 Issue、日志、聊天或版本库。

## 配置与运行

仓库内示例是 [client.toml](examples/client.toml) 与 [server.toml](examples/server.toml)。主要默认值如下：

| 配置项 | 默认值 | 用途 |
| --- | --- | --- |
| `listen` | `0.0.0.0:5260` | 服务端隧道监听地址 |
| `server_addr` | `192.0.2.1:5260` | 仅供文档展示的客户端目标，必须替换 |
| `dns_upstream` | `1.1.1.1:53` | 隧道内 53 端口流量的服务端目标 |
| `proxy_port` | `15260` | 本机透明 TCP/UDP 监听端口 |
| `max_streams` | `256` | 最大多路复用子流数 |
| `udp_idle_secs` | `60` | UDP 关联空闲超时 |
| `reconnect_min_secs` / `reconnect_max_secs` | `1` / `30` | 客户端重连边界 |

放行入站 TCP 5260 后以前台方式启动服务端：

```bash
bash scripts/run-server.sh
# RUST_LOG=debug bash scripts/run-server.sh
```

源码部署时，将 `server.txt.example` 复制为已忽略的 `server.txt`，替换目标并配置 SSH 密钥认证：

```bash
cp server.txt.example server.txt
bash scripts/deploy-client.sh server.txt
ssh -p 22222 root@192.0.2.10 systemctl status canis-client
```

目标机必须提供 `nft`、`ip`、systemd、Linux TPROXY 支持以及 root/CAP_NET_ADMIN。SSH 解析器只接受一个 `ssh [-p PORT] user@IPv4` 目标，强制批处理认证，将首次主机密钥保存到已忽略的 `.canis/known_hosts`，后续密钥变化会被拒绝。

## 流量边界

- 只截获 nftables `output` 路径上的主机进程 IPv4 TCP/UDP。
- 容器网络命名空间经 `forward` 路径转发的流量不在保证范围；`dockerd`、`containerd` 等主机进程自身的流量在范围内。
- 回环、广播、多播、链路本地、DHCP、隧道服务器连接和入站连接的回复方向均绕过代理。
- ICMP、IPv6 和 TCP/UDP 以外协议保持系统原有行为。
- 隧道内任何目标为 TCP 或 UDP 53 的流量都会被服务端改写到 `dns_upstream`。

## 安全模型

隧道通过单个 32 字节 PSK 提供加密和基于持有关系的认证，不提供逐客户端身份或自动密钥轮换。任何持有 PSK 的主体都可认证；服务端同一时间只允许一个已认证会话，新会话会替换旧会话。应保护静态 PSK、仅通过已认证通道传输，并在疑似泄露后手动轮换。

服务端是出站中继：被攻陷且已认证的客户端可以要求它连接任意可达的 IPv4 TCP/UDP 目标。因此应限制服务端网络可达范围和防火墙规则。Canis Minor 不是匿名服务、TLS 检查代理，也不能替代终端加固。详见[架构与安全文档](https://zenithinc.github.io/canis-minor/zh-cn/concepts/architecture/)。

安全漏洞请按 [SECURITY.md](SECURITY.md) 私密报告，不要为尚未公开的漏洞创建公开 Issue。

## 故障恢复与卸载

首次部署时请保留独立 SSH 会话和云平台控制台。服务端不可用时，客户端会先删除 `inet canis_minor`、策略规则和本地路由，再让流量回到主机原有直连路径。可用以下命令检查：

```bash
sudo nft list table inet canis_minor
ip -4 rule show
ip -4 route show table 5260
```

卸载源码部署：

```bash
bash scripts/uninstall-client.sh server.txt
```

远程自动化不可用时，应先执行 `canis-client cleanup --config /etc/canis-minor/client.toml`，再移除服务或二进制。完整步骤见[故障排查文档](https://zenithinc.github.io/canis-minor/zh-cn/operations/troubleshooting/)。

## 开发

```bash
make test     # 19 个 Rust 测试及 SSH 目标解析测试
make check    # rustfmt 与禁止警告的 Clippy
```

真实 TPROXY 验收步骤及匿名化 Ubuntu 24.04 结果见 [ACCEPTANCE.zh-CN.md](ACCEPTANCE.zh-CN.md)。官网位于 `website/`，使用 Node 24 与 pnpm。

欢迎贡献。提交 PR 前请阅读 [CONTRIBUTING.md](CONTRIBUTING.md)、[行为准则](CODE_OF_CONDUCT.md)、[SUPPORT.md](SUPPORT.md) 和[中文路线图](ROADMAP.zh-CN.md)。

## 许可

Copyright 2026 ZenithInc。使用者可任选 [MIT License](LICENSE-MIT) 或 [Apache License 2.0](LICENSE-APACHE)。
