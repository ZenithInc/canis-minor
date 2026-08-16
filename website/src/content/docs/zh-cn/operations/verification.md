---
title: 校验 Release 制品
description: 验证 Canis Minor 校验和、GitHub 构建来源、归档内容与静态二进制。
---

校验将下载字节关联到带标签的 GitHub Actions 构建，但不能替代源码审查或端点安全。

## SHA-256

从同一 Release 下载两个归档与 `SHA256SUMS`，执行：

```bash
sha256sum --check SHA256SUMS
```

目录中每个列出的文件都必须显示 `OK`。不要使用从不可信镜像复制的校验和。

## GitHub 构建来源证明

使用已认证且为当前版本的 GitHub CLI：

```bash
gh attestation verify canis-client-v0.1.0-x86_64-unknown-linux-musl.tar.gz \
  --repo ZenithInc/canis-minor
gh attestation verify canis-server-v0.1.0-x86_64-unknown-linux-musl.tar.gz \
  --repo ZenithInc/canis-minor
gh attestation verify SHA256SUMS --repo ZenithInc/canis-minor
```

确认验证的 subject digest 与本地文件相同，源码仓库是 `ZenithInc/canis-minor`，工作流身份指向标签触发的 Release 工作流。离线流程可参照 GitHub CLI 文档预先下载 attestation。

## 不解压检查

```bash
tar --list --verbose --file canis-client-v0.1.0-x86_64-unknown-linux-musl.tar.gz
tar --list --verbose --file canis-server-v0.1.0-x86_64-unknown-linux-musl.tar.gz
```

应拒绝包含绝对路径、`..` 穿越、异常设备节点或单一版本顶层目录之外文件的归档。

## 验证可执行文件

解压到临时目录后检查：

```bash
artifact_dir=$(mktemp -d)
tar --extract --gzip \
  --file canis-client-v0.1.0-x86_64-unknown-linux-musl.tar.gz \
  --directory "$artifact_dir"
file "$artifact_dir"/*/canis-client
ldd "$artifact_dir"/*/canis-client || true
"$artifact_dir"/*/canis-client --version
"$artifact_dir"/*/canis-client --help
```

`file` 必须识别为 x86-64 静态 PIE。`ldd` 应报告静态链接（措辞可能不同），而不是列出动态库。版本必须与标签和归档名完全一致。

检查后删除临时目录。在完整性、来源、目标架构和归档内容均符合策略前，不要执行二进制。
