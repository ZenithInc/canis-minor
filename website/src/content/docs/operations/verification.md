---
title: Verify release artifacts
description: Verify Canis Minor checksums, GitHub build provenance, archive contents, and static binaries.
---

Verification connects the downloaded bytes to a tagged GitHub Actions build. It does not replace source review or endpoint security.

## SHA-256

Download both archives and `SHA256SUMS` from the same release, then run:

```bash
sha256sum --check SHA256SUMS
```

Every listed file present in the directory must report `OK`. Do not use a checksum copied from an untrusted mirror.

## GitHub build provenance

With an authenticated current GitHub CLI:

```bash
gh attestation verify canis-client-v0.1.0-x86_64-unknown-linux-musl.tar.gz \
  --repo ZenithInc/canis-minor
gh attestation verify canis-server-v0.1.0-x86_64-unknown-linux-musl.tar.gz \
  --repo ZenithInc/canis-minor
gh attestation verify SHA256SUMS --repo ZenithInc/canis-minor
```

Confirm the verified subject digest matches your file, the source repository is `ZenithInc/canis-minor`, and the workflow identity points to the tagged release workflow. See GitHub CLI documentation for downloading attestations when an offline verification process is required.

## Inspect without extracting

```bash
tar --list --verbose --file canis-client-v0.1.0-x86_64-unknown-linux-musl.tar.gz
tar --list --verbose --file canis-server-v0.1.0-x86_64-unknown-linux-musl.tar.gz
```

Reject archives containing absolute paths, `..` traversal, unexpected device nodes, or files outside the single versioned top-level directory.

## Verify the executable

Extract into a temporary directory, then inspect:

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

`file` must identify an x86-64 static PIE executable. `ldd` should report that it is statically linked (wording varies) rather than list dynamic libraries. The version must exactly match the tag and archive name.

Delete the temporary directory after review. Do not execute a binary until integrity, provenance, target architecture, and archive contents all meet your policy.
