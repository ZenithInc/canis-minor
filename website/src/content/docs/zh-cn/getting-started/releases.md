---
title: 使用 Release
description: 下载、验证、检查并安装 Canis Minor Release 制品。
---

每个 `vX.Y.Z` GitHub Release 提供两个 Linux x86_64 musl 归档：

- `canis-client-vX.Y.Z-x86_64-unknown-linux-musl.tar.gz`
- `canis-server-vX.Y.Z-x86_64-unknown-linux-musl.tar.gz`

Release 同时提供 `SHA256SUMS`。GitHub 为两个归档及校验和文件保存构建来源证明。

## 下载与验证

显式设置版本并从对应标签下载：

```bash
version=v0.1.0
curl --fail --location --remote-name \
  "https://github.com/ZenithInc/canis-minor/releases/download/${version}/canis-client-${version}-x86_64-unknown-linux-musl.tar.gz"
curl --fail --location --remote-name \
  "https://github.com/ZenithInc/canis-minor/releases/download/${version}/canis-server-${version}-x86_64-unknown-linux-musl.tar.gz"
curl --fail --location --remote-name \
  "https://github.com/ZenithInc/canis-minor/releases/download/${version}/SHA256SUMS"
sha256sum --check SHA256SUMS
```

使用 GitHub CLI 验证来源：

```bash
gh attestation verify "canis-client-${version}-x86_64-unknown-linux-musl.tar.gz" \
  --repo ZenithInc/canis-minor
gh attestation verify "canis-server-${version}-x86_64-unknown-linux-musl.tar.gz" \
  --repo ZenithInc/canis-minor
gh attestation verify SHA256SUMS --repo ZenithInc/canis-minor
```

离线检查与静态 PIE 验证见[制品校验](../../operations/verification/)。

## 检查归档

解压前先列出文件：

```bash
tar --list --file "canis-client-${version}-x86_64-unknown-linux-musl.tar.gz"
tar --list --file "canis-server-${version}-x86_64-unknown-linux-musl.tar.gz"
```

客户端归档包含二进制、配置示例、systemd 单元、本机安装/卸载材料及两个许可文本。服务端归档包含二进制、配置示例、前台运行辅助脚本及两个许可文本。

解压到新目录，并在安装前阅读内附 `README.md`：

```bash
mkdir "canis-client-${version}"
tar --extract --gzip \
  --file "canis-client-${version}-x86_64-unknown-linux-musl.tar.gz" \
  --directory "canis-client-${version}" --strip-components=1
less "canis-client-${version}/README.md"
```

Release 二进制只面向 Linux x86_64，不要在 macOS、ARM64 或 Windows 上运行。
