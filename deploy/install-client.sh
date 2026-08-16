#!/usr/bin/env bash
set -euo pipefail

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
binary="$script_dir/canis-client"
service="$script_dir/canis-client.service"
config=""
psk=""

usage() {
    echo "usage: $0 --config PATH --psk PATH [--binary PATH] [--service PATH]" >&2
}

while (($#)); do
    case "$1" in
        --binary|--service|--config|--psk)
            (($# >= 2)) || {
                usage
                exit 2
            }
            case "$1" in
                --binary) binary=$2 ;;
                --service) service=$2 ;;
                --config) config=$2 ;;
                --psk) psk=$2 ;;
            esac
            shift 2
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            usage
            exit 2
            ;;
    esac
done

[[ ${EUID} -eq 0 ]] || {
    echo "install-client.sh must run as root" >&2
    exit 1
}
[[ -n $config && -n $psk ]] || {
    echo "--config and --psk are required" >&2
    usage
    exit 2
}
for required in "$binary" "$service" "$config" "$psk"; do
    [[ -f $required ]] || {
        echo "missing install input: $required" >&2
        exit 1
    }
done
[[ -x $binary ]] || {
    echo "client binary is not executable: $binary" >&2
    exit 1
}
[[ $(wc -c <"$psk") -eq 32 ]] || {
    echo "PSK must contain exactly 32 raw bytes" >&2
    exit 1
}
for required_command in nft ip systemctl; do
    command -v "$required_command" >/dev/null 2>&1 || {
        echo "required command is missing: $required_command" >&2
        exit 1
    }
done

if systemctl is-active --quiet canis-client.service; then
    systemctl stop canis-client.service
fi
if [[ -x /usr/local/bin/canis-client && -f /etc/canis-minor/client.toml ]]; then
    /usr/local/bin/canis-client cleanup --config /etc/canis-minor/client.toml || true
fi

install -d -o root -g root -m 0700 /etc/canis-minor
install -o root -g root -m 0755 "$binary" /usr/local/bin/canis-client
install -o root -g root -m 0600 "$psk" /etc/canis-minor/psk
install -o root -g root -m 0600 "$config" /etc/canis-minor/client.toml
install -o root -g root -m 0644 "$service" /etc/systemd/system/canis-client.service

modprobe nf_tproxy_ipv4 2>/dev/null || true
modprobe nft_tproxy 2>/dev/null || true
systemctl daemon-reload
systemctl enable --now canis-client.service
echo "canis-client installed; inspect with: systemctl status canis-client"
