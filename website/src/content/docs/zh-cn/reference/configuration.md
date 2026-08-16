---
title: 配置参考
description: Canis Minor 0.1 客户端与服务端 TOML 完整配置参考。
---

Canis Minor 读取严格 TOML，未知字段会导致错误。IPv4 套接字地址使用带引号的 `IPv4:PORT` 形式；时长均为整数秒。

## 客户端

```toml
server_addr = "192.0.2.1:5260"
psk_file = "/etc/canis-minor/psk"
proxy_port = 15260
fwmark = 13259360
bypass_mark = 13259361
route_table = 5260
rule_priority = 10526
nft_table = "canis_minor"
handshake_timeout_secs = 10
heartbeat_secs = 5
udp_idle_secs = 60
reconnect_min_secs = 1
reconnect_max_secs = 30
max_streams = 256
```

| 字段                     | 必填/默认值   | 约束与行为                                                               |
| ------------------------ | ------------- | ------------------------------------------------------------------------ |
| `server_addr`            | 必填          | 具体 IPv4 地址及非零 TCP 端口，拒绝 `0.0.0.0`。                          |
| `psk_file`               | 必填          | 指向恰好 32 个原始字节；Unix 下不能有 group/other 权限位。               |
| `proxy_port`             | `15260`       | 非零本机透明 TCP/UDP 监听端口。                                          |
| `fwmark`                 | `13259360`    | 用于策略路由的非零截获流量标记。                                         |
| `bypass_mark`            | `13259361`    | 非零且必须与 `fwmark` 不同；标记自身出站套接字以避免递归。               |
| `route_table`            | `5260`        | 正数策略路由表号。                                                       |
| `rule_priority`          | `10526`       | 正数 IPv4 规则优先级。                                                   |
| `nft_table`              | `canis_minor` | 只能含非空 ASCII 字母、数字和下划线；整个 `inet` 表由 Canis Minor 拥有。 |
| `handshake_timeout_secs` | `10`          | 正数 Noise 握手截止时间。                                                |
| `heartbeat_secs`         | `5`           | 驱动 Yamux 健康/保活活动的正数间隔。                                     |
| `udp_idle_secs`          | `60`          | 客户端 UDP 五元组的正数空闲生命周期。                                    |
| `reconnect_min_secs`     | `1`           | 正数初始重试延迟。                                                       |
| `reconnect_max_secs`     | `30`          | 不得小于最小值；指数退避以此为上限。                                     |
| `max_streams`            | `256`         | 1 至 256（含），也限制本机并发代理工作。                                 |

不要让 `proxy_port`、两个 mark、路由表、规则优先级或 nft 表名与其他主机网络组件共用。同一主机多实例应使用唯一值并谨慎测试清理。

## 服务端

```toml
listen = "0.0.0.0:5260"
psk_file = ".canis/psk"
dns_upstream = "1.1.1.1:53"
handshake_timeout_secs = 10
connect_timeout_secs = 10
udp_idle_secs = 60
heartbeat_secs = 5
max_streams = 256
```

| 字段                     | 必填/默认值  | 约束与行为                                                       |
| ------------------------ | ------------ | ---------------------------------------------------------------- |
| `listen`                 | 必填         | IPv4 绑定地址与非零 TCP 端口。                                   |
| `psk_file`               | 必填         | 与客户端相同、私有权限的 32 字节原始 PSK。                       |
| `dns_upstream`           | `1.1.1.1:53` | IPv4 上游，端口必须为 53；隧道内所有 TCP/UDP 53 目标均改写至此。 |
| `handshake_timeout_secs` | `10`         | 正数 Noise 握手截止时间。                                        |
| `connect_timeout_secs`   | `10`         | 服务端发起 TCP 目标连接的正数截止时间。                          |
| `udp_idle_secs`          | `60`         | 服务端 UDP 中继的正数空闲生命周期。                              |
| `heartbeat_secs`         | `5`          | 正数 Yamux 驱动间隔。                                            |
| `max_streams`            | `256`        | 1 至 256（含）。                                                 |

服务端最多接受 64 个待处理握手，并且只保留一个已认证隧道会话：新认证会话会取消旧会话。

## PSK 生命周期

只使用密码学安全随机源生成：

```bash
umask 077
openssl rand 32 > psk
chmod 0600 psk
test "$(wc -c < psk)" -eq 32
```

该文件是二进制，不是十六进制或 Base64 文本。应通过已认证的保密通道分发。轮换时停止客户端、更新两端、重启服务端，再启动客户端并重复验收。0.1 不支持多密钥重叠宽限期。

## 日志

日志由 `RUST_LOG` 控制，默认级别是 `info`：

```bash
RUST_LOG=debug canis-server --config /etc/canis-minor/server.toml
RUST_LOG=canis_client=debug canis-client run --config /etc/canis-minor/client.toml
```

即使 PSK 不会被有意记录，日志仍可能包含敏感运维信息。分享前应匿名化对端与目标地址。
