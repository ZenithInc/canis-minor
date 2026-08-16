#!/usr/bin/env bash
set -euo pipefail

if [[ ${EUID} -ne 0 ]]; then
    echo "remote-uninstall.sh must run as root" >&2
    exit 1
fi

systemctl disable --now canis-client.service 2>/dev/null || true
if [[ -x /usr/local/bin/canis-client && -f /etc/canis-minor/client.toml ]]; then
    /usr/local/bin/canis-client cleanup --config /etc/canis-minor/client.toml || true
fi
rm -f -- /etc/systemd/system/canis-client.service /usr/local/bin/canis-client
rm -rf -- /etc/canis-minor
systemctl daemon-reload
systemctl reset-failed canis-client.service 2>/dev/null || true
echo "canis-client uninstalled; its nftables table and policy route were removed"
