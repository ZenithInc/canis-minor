# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-08-16

### Added

- Linux x86_64 transparent IPv4 TCP/UDP client using nftables TPROXY and policy routing.
- Encrypted PSK-authenticated Noise NNpsk0 transport multiplexed with Yamux.
- Linux relay server with configurable DNS redirection for tunneled port 53 traffic.
- Fail-open cleanup, bounded automatic reconnection, systemd deployment, and idempotent uninstall tooling.
- Static musl release artifacts, checksums, build provenance, bilingual documentation, and project website.
- Nineteen Rust tests plus strict SSH target-parser coverage.

[Unreleased]: https://github.com/ZenithInc/canis-minor/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/ZenithInc/canis-minor/releases/tag/v0.1.0
