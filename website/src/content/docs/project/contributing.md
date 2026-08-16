---
title: Contributing
description: Develop, test, document, and propose changes to Canis Minor.
---

Contributions are welcome under the repository's `MIT OR Apache-2.0` license. Participation is governed by the [Contributor Covenant](https://github.com/ZenithInc/canis-minor/blob/main/CODE_OF_CONDUCT.md).

## Find the right channel

- Usage and design questions: [GitHub Discussions](https://github.com/ZenithInc/canis-minor/discussions).
- Reproducible defects and scoped proposals: Issue Forms.
- Undisclosed vulnerabilities: [private vulnerability reporting](https://github.com/ZenithInc/canis-minor/security/advisories/new).
- Large protocol, routing, platform, or security work: discuss before implementation.

## Validate code changes

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
bash tests/target-parser.sh
```

Changes to privileged networking require the [real-host acceptance procedure](../acceptance/) and anonymized evidence.

## Validate website changes

Use Node 24 and pnpm 11:

```bash
cd website
corepack enable
pnpm install --frozen-lockfile
pnpm format:check
pnpm check
pnpm build
pnpm check:links
```

Update English and Simplified Chinese pages together for user-visible behavior. Use relative or `/canis-minor/`-prefixed project links so GitHub Pages base-path hosting continues to work.

## Protect private data

Never commit `.canis/`, `server.txt`, a PSK, known-hosts data, build outputs, or live addresses. Use RFC 5737 documentation networks. Redact logs even when they do not contain key material.

Read the complete [CONTRIBUTING.md](https://github.com/ZenithInc/canis-minor/blob/main/CONTRIBUTING.md) and complete the pull request template. Required checks must pass and review conversations must be resolved; merges are squash-only.
