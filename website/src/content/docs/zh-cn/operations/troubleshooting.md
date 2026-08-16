---
title: 故障排查与恢复
description: 诊断隧道、TPROXY、DNS、路由及断线直连恢复问题。
---

安全优先：保留独立管理连接与云平台控制台，不要在共享主机上清空全部 nftables 或路由状态。

## 收集匿名化快照

```bash
systemctl status canis-client --no-pager
journalctl -u canis-client --since=-15m --no-pager
sudo nft list table inet canis_minor
ip -4 rule show
ip -4 route show table 5260
ss -lntup | grep 15260
```

分享输出前移除 PSK、用户名、SSH 目标、主机密钥与真实公网/私网基础设施地址。

## 客户端始终不启用截获

客户端只有在以下条件全部成功后才安装规则：

1. 使用 bypass mark 直连 `server_addr`。
2. 使用匹配 PSK 完成 Noise 握手。
3. 绑定透明 TCP 监听器。
4. 绑定透明 UDP 监听器。
5. 启动 Yamux。

检查服务端可达性、防火墙、两端 PSK 是否一致、文件长度/权限、端口冲突、root 权限及内核模块。错误 PSK 会表现为握手失败，而不会部分启用。

## PSK 错误

```bash
wc -c /etc/canis-minor/psk
stat -c '%a %U:%G %n' /etc/canis-minor/psk
sha256sum /etc/canis-minor/psk  # 只通过可信管理通道比较
```

密钥必须是 32 个原始字节，不是 64 个十六进制字符或 Base64 文本，且 group/other 不可访问。其摘要也可能关联不同部署，不应公开。

## 崩溃后规则残留

systemd 的 `ExecStartPre` 与 `ExecStopPost` 会调用幂等清理，必要时可手动执行：

```bash
sudo /usr/local/bin/canis-client cleanup \
  --config /etc/canis-minor/client.toml
```

随后检查配置对应的表与路由对象。不要删除宽泛的系统规则。如果先删除了配置，应恢复包含完全相同标识符的临时配置，或通过控制台谨慎地只删除已知 Canis Minor 对象。

## 流量循环或无法联网

确认 `fwmark` 与 `bypass_mark` 不同，路由表和优先级不与其他策略系统冲突，且 `server_addr` 是实际具体 IPv4 端点。验证服务端连接与入站回复流量匹配绕过规则。与 VPN、其他透明代理或主机管理代理冲突时，需要分配唯一 mark/表/优先级并重新验收。

## DNS 行为异常

Canis Minor 只改写隧道内 TCP/UDP 目标端口 53：

```bash
getent ahostsv4 registry-1.docker.io
dig +tcp @1.1.1.1 example.com
dig @1.1.1.1 example.com
```

浏览器安全 DNS、DNS-over-TLS、IPv6 与容器命名空间解析器可能走其他路径。服务端 `dns_upstream` 必须使用 53 端口并可由服务端访问。

## 隧道反复重连

检查两端日志中的握手、加密、Yamux、心跳、子流上限和目标错误。延迟从 `reconnect_min_secs` 翻倍到 `reconnect_max_secs`；持续至少 30 秒的会话会重置为最小值。检查中间设备 TCP 空闲超时以及服务端替换会话——第二个已认证客户端会有意断开第一个。

## 紧急恢复

1. 使用独立控制台或 SSH 会话。
2. 执行 `systemctl stop canis-client`。
3. 执行上面的显式清理命令。
4. 确认配置的 nft 表、策略规则和路由均已消失。
5. 测试 IPv4 直连。
6. 在理解配置与冲突前保持服务禁用。

若故障可能是漏洞，请使用[私密报告](https://github.com/ZenithInc/canis-minor/security/advisories/new)，不要创建公开 Issue。
