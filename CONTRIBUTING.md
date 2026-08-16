# Contributing to Canis Minor

Thank you for helping improve Canis Minor. By participating, you agree to follow the [Code of Conduct](CODE_OF_CONDUCT.md).

## Before you start

- Use GitHub Discussions for design questions and support requests.
- Search existing issues before opening a bug or feature request.
- Report security vulnerabilities privately according to [SECURITY.md](SECURITY.md).
- Open an issue before making a large protocol, routing, platform, or security change so maintainers can confirm direction.

## Development environment

The supported environment is Ubuntu 24.04 x86_64 with Rust 1.85.0. The website requires Node 24 and pnpm 11.

```bash
git clone git@github.com:YOUR_ACCOUNT/canis-minor.git
cd canis-minor
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
bash tests/target-parser.sh
```

For website changes:

```bash
cd website
corepack enable
pnpm install --frozen-lockfile
pnpm format:check
pnpm check
pnpm build
pnpm check:links
```

Never commit `.canis/`, `server.txt`, PSKs, SSH known-hosts data, build outputs, or real infrastructure addresses. Use RFC 5737 addresses (`192.0.2.0/24`, `198.51.100.0/24`, or `203.0.113.0/24`) in examples.

## Pull requests

Create a focused branch, add tests for behavior changes, update both English and Simplified Chinese documentation when user-visible behavior changes, and complete the pull request template. Keep commits understandable; the repository uses squash merging.

All required checks must pass and review conversations must be resolved. The project does not require a contributor license agreement. Contributions are accepted under the repository's `MIT OR Apache-2.0` license.

## Testing privileged networking changes

Unit tests do not prove kernel TPROXY behavior. For changes to nftables, routing, socket transparency, DNS, cleanup, or reconnection, include an anonymized acceptance report following [ACCEPTANCE.md](ACCEPTANCE.md). Keep an independent management session and provider console available during real-host tests.

## Release process

Maintainers update `CHANGELOG.md`, keep all Cargo workspace packages at the intended semantic version, merge through `main`, and create an annotated `vX.Y.Z` tag. The release workflow rejects tags that do not match the Cargo workspace version.
