---
title: 卸载
description: 不遗留 nftables、路由、服务或秘密状态地移除 Canis Minor。
---

删除客户端配置或二进制之前，必须先清理网络状态。

## 使用仓库辅助卸载

在含已忽略 `server.txt` 与 `.canis/known_hosts` 的源码检出中执行：

```bash
bash scripts/uninstall-client.sh server.txt
```

脚本上传范围受限的卸载辅助程序，停止并禁用服务，在配置存在时执行幂等清理，移除安装文件，重载 systemd 并清除失败状态。

## 本机卸载

在客户端主机执行：

```bash
sudo systemctl disable --now canis-client.service
sudo /usr/local/bin/canis-client cleanup \
  --config /etc/canis-minor/client.toml
sudo rm -f /etc/systemd/system/canis-client.service
sudo rm -f /usr/local/bin/canis-client
sudo rm -rf /etc/canis-minor
sudo systemctl daemon-reload
sudo systemctl reset-failed canis-client.service || true
```

以上路径具体且有意限定。若部署使用自定义位置，执行前必须检查。

## 验证移除

```bash
! systemctl cat canis-client.service
! sudo nft list table inet canis_minor
! test -e /etc/canis-minor
! test -e /usr/local/bin/canis-client
ip -4 rule show
ip -4 route show table 5260
```

最后两个命令不应包含匹配所配置优先级/mark/表的对象。若自定义或共用了标识符，不能假定路由表 5260 中的所有条目都属于 Canis Minor。

## 移除源码侧秘密

确认没有端点再需要它们后，按组织的秘密保留策略安全退役 `.canis/psk`、生成的 TOML 和 `.canis/known_hosts`。`server.txt` 与 `.canis/` 虽被 Git 忽略，仍然是敏感本地文件。

移除服务端更简单：停止其服务管理器，删除二进制/配置，并退役共享 PSK。服务端不会创建 nftables 或策略路由状态。
