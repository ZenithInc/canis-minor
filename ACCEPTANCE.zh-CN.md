# 实机验收记录（2026-08-16）

[English](ACCEPTANCE.md)

环境：以 RFC 5737 IPv4 地址 `203.0.113.10` 表示的匿名化服务端；Ubuntu 24.04 x86_64 目标机。验收期间始终保留独立 SSH 会话。

## 网络与 Registry

- 目标机观测到的公网出口与 `203.0.113.10` 一致。
- IPv4 HTTP 与 HTTPS 请求成功，状态码为 `200`。
- `registry-1.docker.io` 未解析到测试夹具使用的已知错误地址。
- Docker Registry `/v2/` 按预期为未认证请求返回 `401`。
- Docker 鉴权接口返回 `200`。
- TCP 与 UDP 目标端口 53 都进入隧道，并由服务端改写到配置的 `1.1.1.1:53` 上游。

## APT 与 Docker

- `apt-get update`：38 秒。
- 安装 Ubuntu `docker.io` 及依赖：197 秒。
- Docker `29.1.3-0ubuntu3~24.04.2` 服务达到 `active` 状态。
- `alpine:latest`：24 秒，首次成功。
- `ubuntu:24.04`：78 秒，首次成功。

以上耗时作为匿名化运维基线保留，不构成性能保证。

## 故障恢复

- 停止服务端后 1 秒内，客户端移除了 nftables 表、策略规则和两个透明监听器。
- 连续执行两次 `canis-client cleanup` 均成功，证明清理幂等。
- 服务端恢复后 4 秒内自动重连，公网出口再次与 `203.0.113.10` 一致。
- 重连后 TCP/UDP 透明监听器各一个，无残留监听器、无 panic，systemd 报告 `NRestarts=0`。
- 最终状态：`canis-client.service=active`、`docker.service=active`。

远端客户端二进制与本地静态构建产物的 SHA-256 一致：

```text
d4e540553fe90162e93758bc98258520c2f38d175be910ec1b90ac4421e297bd
```

## 重复验收

保留独立管理会话和云平台控制台，依次验证公网出口、HTTP/HTTPS、TCP/UDP DNS、APT、镜像拉取、隧道中断、幂等清理、有上限重连，以及不存在残留 nftables、路由或监听器状态。精确命令见[在线验收指南](https://zenithinc.github.io/canis-minor/zh-cn/project/acceptance/)。
