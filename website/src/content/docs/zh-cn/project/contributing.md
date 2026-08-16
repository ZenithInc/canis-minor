---
title: 贡献
description: 开发、测试、记录并向 Canis Minor 提议改动。
---

欢迎在仓库 `MIT OR Apache-2.0` 许可下贡献。参与行为受 [Contributor Covenant](https://github.com/ZenithInc/canis-minor/blob/main/CODE_OF_CONDUCT.md) 约束。

## 选择正确渠道

- 使用与设计问题：[GitHub Discussions](https://github.com/ZenithInc/canis-minor/discussions)。
- 可复现缺陷与范围明确的提案：Issue Forms。
- 未公开漏洞：[私密漏洞报告](https://github.com/ZenithInc/canis-minor/security/advisories/new)。
- 大型协议、路由、平台或安全工作：实现前先讨论。

## 验证代码改动

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
bash tests/target-parser.sh
```

特权网络改动需要执行[真实主机验收](../acceptance/)并提供匿名化证据。

## 验证官网改动

使用 Node 24 与 pnpm 11：

```bash
cd website
corepack enable
pnpm install --frozen-lockfile
pnpm format:check
pnpm check
pnpm build
pnpm check:links
```

面向用户的行为变化应同时更新英文与简体中文页面。使用相对链接或以 `/canis-minor/` 开头的项目链接，保证 GitHub Pages base path 正常。

## 保护私密数据

不得提交 `.canis/`、`server.txt`、PSK、known-hosts 数据、构建输出或真实地址。示例使用 RFC 5737 文档网段；即使日志不含密钥也应匿名化。

请阅读完整的 [CONTRIBUTING.md](https://github.com/ZenithInc/canis-minor/blob/main/CONTRIBUTING.md) 并填写 PR 模板。required checks 必须通过，评审会话必须解决；仓库仅使用 squash merge。
