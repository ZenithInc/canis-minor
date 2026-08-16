---
title: Use a release
description: Download, verify, inspect, and install Canis Minor release artifacts.
---

Each `vX.Y.Z` GitHub release provides two Linux x86_64 musl archives:

- `canis-client-vX.Y.Z-x86_64-unknown-linux-musl.tar.gz`
- `canis-server-vX.Y.Z-x86_64-unknown-linux-musl.tar.gz`

The release also provides `SHA256SUMS`. GitHub stores build provenance attestations for both archives and the checksum file.

## Download and verify

Set the version explicitly and download from the matching tag:

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

Verify provenance with GitHub CLI:

```bash
gh attestation verify "canis-client-${version}-x86_64-unknown-linux-musl.tar.gz" \
  --repo ZenithInc/canis-minor
gh attestation verify "canis-server-${version}-x86_64-unknown-linux-musl.tar.gz" \
  --repo ZenithInc/canis-minor
gh attestation verify SHA256SUMS --repo ZenithInc/canis-minor
```

See [artifact verification](../../operations/verification/) for offline inspection and static PIE checks.

## Inspect the archives

List files before extraction:

```bash
tar --list --file "canis-client-${version}-x86_64-unknown-linux-musl.tar.gz"
tar --list --file "canis-server-${version}-x86_64-unknown-linux-musl.tar.gz"
```

The client archive includes the binary, example configuration, systemd unit, local install/uninstall materials, and both license texts. The server archive includes the binary, example configuration, a foreground run helper, and both license texts.

Extract into a new directory and read the included `README.md` before installing:

```bash
mkdir "canis-client-${version}"
tar --extract --gzip \
  --file "canis-client-${version}-x86_64-unknown-linux-musl.tar.gz" \
  --directory "canis-client-${version}" --strip-components=1
less "canis-client-${version}/README.md"
```

Release binaries target Linux x86_64 only. Do not run them on macOS, ARM64, or Windows.
