#!/usr/bin/env bash
set -euo pipefail

if [[ ${EUID} -ne 0 ]]; then
    echo "remote-install.sh must run as root" >&2
    exit 1
fi

source_dir=${1:?usage: remote-install.sh UPLOAD_DIRECTORY}
case "$source_dir" in
    /tmp/canis-minor.*) ;;
    *)
        echo "refusing unexpected upload directory: $source_dir" >&2
        exit 1
        ;;
esac

for command in nft ip systemctl; do
    if ! command -v "$command" >/dev/null 2>&1; then
        echo "required command is missing: $command" >&2
        exit 1
    fi
done

if systemctl is-active --quiet canis-client.service; then
    systemctl stop canis-client.service
fi
if [[ -x /usr/local/bin/canis-client && -f /etc/canis-minor/client.toml ]]; then
    /usr/local/bin/canis-client cleanup --config /etc/canis-minor/client.toml || true
fi

install -d -o root -g root -m 0700 /etc/canis-minor
install -o root -g root -m 0755 "$source_dir/canis-client" /usr/local/bin/canis-client
install -o root -g root -m 0600 "$source_dir/psk" /etc/canis-minor/psk
install -o root -g root -m 0600 "$source_dir/client.toml" /etc/canis-minor/client.toml
install -o root -g root -m 0644 "$source_dir/canis-client.service" /etc/systemd/system/canis-client.service

modprobe nf_tproxy_ipv4 2>/dev/null || true
modprobe nft_tproxy 2>/dev/null || true
systemctl daemon-reload
systemctl enable --now canis-client.service
rm -rf -- "$source_dir"

echo "canis-client installed; inspect with: systemctl status canis-client"
