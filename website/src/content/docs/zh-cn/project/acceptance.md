---
title: 验收测试
description: 验证真实 Ubuntu TPROXY、DNS、软件包流量、恢复和制品身份。
---

单元测试无法验证真实内核的 TPROXY、nftables、策略路由、防火墙或云网络。每次新部署及相关升级后都应完成此流程。

:::danger
保持独立 SSH 会话与云平台控制台开启。启用 Canis Minor 前记录客户端原始公网 IPv4 与路由状态。
:::

## 基线与活动状态

启动客户端前记录：

```bash
curl -4 --fail --silent https://api.ipify.org; echo
ip -4 rule show
ip -4 route show
```

启动后确认服务、表、规则、路由，以及恰好一个 TCP 与一个 UDP 透明监听器：

```bash
systemctl is-active canis-client
sudo nft list table inet canis_minor
ip -4 rule show
ip -4 route show table 5260
ss -lntup | grep 15260
```

## 出口、Web 与 DNS

将下面 `203.0.113.10` 替换为服务端实际公网出口，并与输出比较：

```bash
test "$(curl -4 --fail --silent https://api.ipify.org)" = "203.0.113.10"
curl -4 --fail --silent --output /dev/null https://example.com/
getent ahostsv4 registry-1.docker.io
curl -4 -sS -o /dev/null -w '%{http_code}\n' https://registry-1.docker.io/v2/
curl -4 -sS -o /dev/null -w '%{http_code}\n' \
  'https://auth.docker.io/token?service=registry.docker.io&scope=repository:library/alpine:pull'
```

Registry 状态应为 `401`，鉴权接口应为 `200`。在服务端记录 UDP 与 TCP 目标端口 53 都到达所配置上游的证据。

## APT 与容器拉取

```bash
time sudo apt-get update
time sudo apt-get install -y docker.io
sudo systemctl enable --now docker
time sudo docker pull alpine:latest
time sudo docker pull ubuntu:24.04
```

这些命令覆盖主机进程（`apt`、`dockerd` 与 `containerd`），不代表任意容器网络命名空间内的流量。

## 断线直连与重连

1. 停止服务端隧道监听器或用防火墙阻断。
2. 在所配置故障检测窗口内，确认 `inet canis_minor`、对应策略规则/路由和透明监听器消失。
3. 确认独立 SSH 会话仍可用，并恢复 IPv4 直连。
4. 连续运行两次 `canis-client cleanup`，确认均成功。
5. 恢复服务端。
6. 确认在上限内自动重连，TCP/UDP 监听器各一个，无遗留规则、无 panic，且代理出口正确。

## 记录证据

记录版本、匿名化拓扑、结果、耗时、故障/恢复时间、`NRestarts`，以及部署二进制与已验证构建之间的 SHA-256 一致性。不得包含 PSK 或真实基础设施标识。

项目初始匿名化记录见 [ACCEPTANCE.zh-CN.md](https://github.com/ZenithInc/canis-minor/blob/main/ACCEPTANCE.zh-CN.md)。
