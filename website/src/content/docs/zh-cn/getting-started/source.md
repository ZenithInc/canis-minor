---
title: 从源码构建
description: 使用固定 Rust 工具链构建和测试 Canis Minor。
---

正式支持的源码构建主机是 Ubuntu 24.04 x86_64。仓库固定 Rust 1.85.0，并使用 2024 edition。

## 安装前置依赖

```bash
sudo apt-get update
sudo apt-get install -y build-essential file openssl pkg-config
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

若 rustup 尚未进入你的可信工具链流程，请在执行前审查其安装程序。

## 克隆与测试

```bash
git clone https://github.com/ZenithInc/canis-minor.git
cd canis-minor
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
bash tests/target-parser.sh
```

Rust 套件包含 19 个测试，覆盖配置约束、PSK 不匹配、密文篡改、协议版本、IPv4 地址编码、UDP 边界、TCP 半关闭、并发子流、目标连接失败、隧道中断、规则生成和恢复。

## 初始化私密材料

服务端地址必填；项目不提供运营中的默认服务器：

```bash
bash scripts/init.sh --server-address 192.0.2.1:5260
```

将 `192.0.2.1` 替换为你的服务端 IPv4。命令写入 `.canis/psk`、`.canis/server.toml` 与 `.canis/client.toml`，这些文件已被 Git 忽略。`--force` 会替换三者并轮换 PSK，使用前必须协调两端。

## 构建

```bash
bash scripts/build-server.sh
bash scripts/build-static.sh
```

`build-static.sh` 安装 `x86_64-unknown-linux-musl` target，并构建客户端与服务端 Release 二进制。确认二者为静态 PIE 且可运行：

```bash
file target/x86_64-unknown-linux-musl/release/canis-client
file target/x86_64-unknown-linux-musl/release/canis-server
ldd target/x86_64-unknown-linux-musl/release/canis-client || true
target/x86_64-unknown-linux-musl/release/canis-client --help
target/x86_64-unknown-linux-musl/release/canis-server --help
```

下一步阅读[配置参考](../../reference/configuration/)与[部署](../../operations/deployment/)。
