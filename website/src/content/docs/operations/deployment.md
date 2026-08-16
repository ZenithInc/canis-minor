---
title: Deploy and upgrade
description: Safely deploy and upgrade the Canis Minor server and Linux client.
---

Use a dedicated test host first. Keep an independent SSH session to the client and a provider console available until failure recovery has been verified.

## Prepare private configuration

From a source checkout:

```bash
bash scripts/init.sh --server-address 192.0.2.1:5260
```

Replace the documentation address with the server's reachable IPv4 address. This creates one PSK and endpoint-specific TOML under ignored `.canis/`. Confirm the key is exactly 32 bytes and all files are private:

```bash
wc -c .canis/psk
stat -c '%a %n' .canis/psk .canis/client.toml .canis/server.toml
```

## Start the server

Build and run in the foreground while first validating:

```bash
bash scripts/build-server.sh
RUST_LOG=info bash scripts/run-server.sh
```

Permit the configured TCP port through host and provider firewalls. Do not expose administrative ports unnecessarily. For long-running operation, place the binary and configuration under your own supervised service with a dedicated working directory and a `0600` PSK.

## Deploy the client from source

Build the static binary, prepare an ignored SSH target, and deploy:

```bash
bash scripts/build-static.sh
cp server.txt.example server.txt
# Edit server.txt to one line: ssh [-p PORT] user@IPv4
bash scripts/deploy-client.sh server.txt
```

The parser does not evaluate shell syntax. Deployment requires SSH key authentication and uses an isolated known-hosts file at `.canis/known_hosts`. The first connection accepts a new host key; later changes fail closed.

Files installed on the client are:

```text
/usr/local/bin/canis-client
/etc/canis-minor/client.toml
/etc/canis-minor/psk
/etc/systemd/system/canis-client.service
```

The remote installer checks `nft`, `ip`, and systemd, stops and cleans any prior service, installs with restrictive permissions, reloads systemd, and enables the client.

## Validate

```bash
systemctl status canis-client --no-pager
journalctl -u canis-client --since=-10m --no-pager
nft list table inet canis_minor
ip -4 rule show
ip -4 route show table 5260
curl -4 --fail --silent https://api.ipify.org; echo
```

Complete the [acceptance procedure](../../project/acceptance/), including stopping the server and confirming direct-path cleanup before calling the deployment complete.

## Upgrade

1. Read the changelog and back up both configurations and the PSK through a secure channel.
2. Download and [verify](../verification/) both new artifacts.
3. Replace and restart the server. Expect the client to remove interception and reconnect.
4. Replace the client using the source deployment script or the verified release install material. The systemd pre/post hooks clean old state.
5. Repeat status, egress, DNS, TCP/UDP, and failure-recovery checks.

Protocol compatibility is not promised across unreleased commits. Upgrade both sides to the same released version.

## Rollback

Stop the client first, run explicit cleanup, restore the previous client binary/configuration, restore the matching server version, and start the server before the client. Reusing the unchanged PSK is operationally possible, but rotate it if rollback relates to suspected compromise.
