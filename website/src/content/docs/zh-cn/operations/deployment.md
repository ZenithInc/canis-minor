---
title: 部署与升级
description: 安全部署和升级 Canis Minor 服务端与 Linux 客户端。
---

首先使用专用测试主机。在完成故障恢复验证前，始终保留客户端的独立 SSH 会话与云平台控制台。

## 准备私密配置

在源码检出中执行：

```bash
bash scripts/init.sh --server-address 192.0.2.1:5260
```

将文档地址替换为服务端可达 IPv4。该命令在已忽略的 `.canis/` 下生成一份 PSK 和端点配置。确认密钥恰好 32 字节且文件权限私有：

```bash
wc -c .canis/psk
stat -c '%a %n' .canis/psk .canis/client.toml .canis/server.toml
```

## 启动服务端

首次验证时构建并以前台方式运行：

```bash
bash scripts/build-server.sh
RUST_LOG=info bash scripts/run-server.sh
```

在主机与云平台防火墙中允许配置的 TCP 端口，不要暴露不必要的管理端口。长期运行时，应将二进制和配置放入自有服务管理器，使用专用工作目录及 `0600` PSK。

## 从源码部署客户端

构建静态二进制、准备已忽略的 SSH 目标并部署：

```bash
bash scripts/build-static.sh
cp server.txt.example server.txt
# 编辑 server.txt，仅保留一行：ssh [-p PORT] user@IPv4
bash scripts/deploy-client.sh server.txt
```

解析器不会执行 shell 语法。部署要求 SSH 密钥认证，并使用 `.canis/known_hosts` 隔离主机密钥。首次连接接受新密钥，后续密钥变化会失败关闭。

客户端安装文件：

```text
/usr/local/bin/canis-client
/etc/canis-minor/client.toml
/etc/canis-minor/psk
/etc/systemd/system/canis-client.service
```

远端安装器检查 `nft`、`ip` 与 systemd，停止并清理旧服务，以严格权限安装，重载 systemd 并启用客户端。

## 验证

```bash
systemctl status canis-client --no-pager
journalctl -u canis-client --since=-10m --no-pager
nft list table inet canis_minor
ip -4 rule show
ip -4 route show table 5260
curl -4 --fail --silent https://api.ipify.org; echo
```

完成[验收流程](../../project/acceptance/)，其中必须停止服务端并确认恢复直连，才能认为部署完成。

## 升级

1. 阅读 Changelog，并通过安全通道备份两端配置与 PSK。
2. 下载并[验证](../verification/)两个新制品。
3. 替换并重启服务端；客户端应移除截获状态并重连。
4. 使用源码部署脚本或已验证的 Release 安装材料替换客户端；systemd 前后置钩子会清理旧状态。
5. 重复状态、出口、DNS、TCP/UDP 与故障恢复检查。

未发布 commit 之间不承诺协议兼容，应将两端升级至相同发布版本。

## 回滚

先停止客户端并显式清理，恢复旧客户端二进制/配置，恢复匹配的服务端版本，然后先启动服务端、再启动客户端。保留原 PSK 在操作上可行，但若回滚与疑似入侵有关，则必须轮换。
