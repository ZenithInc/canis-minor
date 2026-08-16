---
title: Uninstall
description: Remove Canis Minor without leaving nftables, routing, service, or secret state.
---

Always clean network state before deleting the client configuration or binary.

## Repository-assisted uninstall

From the source checkout that contains your ignored `server.txt` and `.canis/known_hosts`:

```bash
bash scripts/uninstall-client.sh server.txt
```

The script uploads a narrowly scoped uninstall helper, stops and disables the service, performs idempotent cleanup when configuration is present, removes installed files, reloads systemd, and clears the failed state.

## Local uninstall

On the client host:

```bash
sudo systemctl disable --now canis-client.service
sudo /usr/local/bin/canis-client cleanup \
  --config /etc/canis-minor/client.toml
sudo rm -f /etc/systemd/system/canis-client.service
sudo rm -f /usr/local/bin/canis-client
sudo rm -rf /etc/canis-minor
sudo systemctl daemon-reload
sudo systemctl reset-failed canis-client.service || true
```

These paths are specific and intentional. Review them before execution if your deployment uses custom locations.

## Verify removal

```bash
! systemctl cat canis-client.service
! sudo nft list table inet canis_minor
! test -e /etc/canis-minor
! test -e /usr/local/bin/canis-client
ip -4 rule show
ip -4 route show table 5260
```

The final two commands should contain no objects matching the configured Canis Minor priority/mark/table. Do not assume every entry in routing table 5260 belongs to Canis Minor if you customized or shared identifiers.

## Remove source-side secrets

After confirming no endpoint needs them, securely retire `.canis/psk`, generated TOML, and `.canis/known_hosts` according to your organization's secret-retention policy. `server.txt` and `.canis/` are Git-ignored but are still sensitive local files.

Removing the server is simpler: stop its supervisor, remove its binary/configuration, and retire the shared PSK. The server creates no nftables or policy-routing state.
