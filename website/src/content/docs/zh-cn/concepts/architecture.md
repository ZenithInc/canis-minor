---
title: 架构与安全模型
description: 理解 Canis Minor 的数据面、信任模型、加密与权限边界。
---

Canis Minor 由三个 Rust crate 组成：`canis-client`、`canis-server` 与共享的 `canis-protocol` 库。只有客户端会改变主机网络状态。

## 会话生命周期

1. 客户端删除与所配置 nft 表、mark、路由表和优先级相符的遗留对象。
2. 带 bypass mark 的 TCP 套接字直连 `server_addr`。
3. 客户端与服务端用相同的 32 字节 PSK 完成 Noise NNpsk0 握手。
4. 客户端绑定透明 TCP 与 UDP 监听器。
5. 在加密传输之上启动 Yamux 会话。
6. 只有此时客户端才安装 nftables 表、策略规则与本地路由。
7. 每个被截获的 TCP 连接或 UDP 五元组都会打开一个多路复用子流，并携带带版本的目标请求。
8. 监听器、复用层、加密层、隧道或关闭事件会取消代理工作并移除截获状态，然后才进入重连。

这种顺序避免在可用的已认证隧道建立之前给主机安装黑洞路由。

## 数据面

```text
应用 TCP/UDP
    │
    ▼
inet canis_minor / output
    │ mark + TPROXY 路由
    ▼
透明监听器
    │ 目标元数据 + 载荷
    ▼
Noise 传输 ─ Yamux 子流 ─ 服务端套接字 ─ 目标
```

TCP 子流保留双向半关闭；UDP 子流保留数据报边界，包括空数据报。两端都强制 `max_streams`，超额工作会被拒绝，而不会无限排队。

## 密码学边界

Noise 提供加密传输并证明共享 PSK 的持有关系。隧道中的 TLS 仍然端到端存在于原始应用与目标之间；Canis Minor 不终止也不检查 TLS。

0.1 没有证书 PKI、命名客户端身份、多客户端授权或自动轮换。因此 PSK 是高价值 bearer secret，获得它的人可以冒充端点。不得使用弱密钥、复用密钥、文本编码密钥或公开分发的密钥。

## 服务端信任与可达性

已认证客户端可以请求访问服务端网络可达的 IPv4 TCP/UDP 目标；除非操作系统或防火墙阻止，否则也包括私网。应将服务端视为高权限网络中继：

- 在专用或适当隔离的主机上运行。
- 入站只允许配置的 TCP 隧道端口。
- 将出站限制在部署所需范围。
- 保护并监控配置与 PSK。
- 不要宣称匿名性：目标可看到服务端出口，运营者可观察元数据。

服务端只允许一个已认证活动会话。新的成功握手会替代旧会话；这是恢复行为，不是多客户端支持。

## 客户端权限边界

透明截获需要 root 或 Linux 网络 capabilities。提供的 systemd 单元限制地址族和 capabilities，启用 `NoNewPrivileges`、保护系统路径并禁止可写可执行内存，但因需管理 nftables 和路由仍以 root 运行。

清理命令只构造配置中属于自己的对象名，并设计为幂等。运维者必须避免选用其他策略路由或 nftables 系统正在使用的标识符。

## 故障模型

安全路径失败时，Canis Minor 选择恢复直连，而不是持续截获。隧道中断可能改变公网出口，并绕过只存在于中继上的控制。若必须阻止直连，应在独立的主机或网络策略中实施；不要把 Canis Minor 的 fail-open 行为当作 kill switch。

部署前请阅读[流量边界](../traffic-boundaries/)，报告漏洞前请阅读[安全策略](https://github.com/ZenithInc/canis-minor/blob/main/SECURITY.md)。
