---
title: Installation overview
description: Choose a Canis Minor installation path and prepare a safe Linux deployment.
---

Canis Minor 0.1 supports Ubuntu 24.04, Linux x86_64, and IPv4. A typical deployment has one public or otherwise reachable server and one client host whose host-originated TCP/UDP traffic should exit through that server.

:::caution
The client changes nftables and policy-routing state. Keep an independent SSH session and provider console available during the first deployment. Do not test first on a host you cannot recover out of band.
:::

## Choose an installation path

- [Use a release](./releases/) for signed GitHub workflow provenance, checksums, and static musl binaries.
- [Build from source](./source/) when auditing or changing the implementation.

No container image, operating-system package repository, or crates.io package is published for 0.1.

## Prerequisites

The server needs a reachable TCP port (5260 by default). The client needs:

- Ubuntu 24.04 x86_64 with a kernel that supports IPv4 TPROXY.
- `nft`, `ip`, and systemd.
- Root or equivalent `CAP_NET_ADMIN`/`CAP_NET_RAW` capabilities.
- Normal direct network connectivity for fail-open operation.
- SSH key authentication if using the repository deployment scripts.

Both sides must hold the same 32-byte raw PSK at mode `0600`. The key is the only authentication secret in 0.1.

## Deployment sequence

1. Build or download both static binaries and verify them.
2. Generate a PSK and configurations with an explicit server address.
3. Start the server and permit its TCP listen port in the firewall.
4. Keep an independent management connection to the client.
5. Install and start the client.
6. Run the [acceptance procedure](../project/acceptance/), including tunnel-loss recovery.

Continue with [deployment and upgrades](../operations/deployment/) for the complete operational procedure.
