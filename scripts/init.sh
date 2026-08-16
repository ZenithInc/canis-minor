#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
server_address=""
force=0

usage() {
    echo "usage: $0 --server-address IPv4:PORT [--force]" >&2
}

while (($#)); do
    case "$1" in
        --server-address)
            if (($# < 2)); then
                usage
                exit 2
            fi
            server_address=$2
            shift 2
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        --force)
            force=1
            shift
            ;;
        *)
            usage
            exit 2
            ;;
    esac
done

[[ -n $server_address ]] || {
    echo "--server-address is required; no default server is configured" >&2
    usage
    exit 2
}
[[ $server_address =~ ^([0-9]{1,3}[.]){3}[0-9]{1,3}:[0-9]{1,5}$ ]] || {
    echo "server address must be numeric IPv4:PORT" >&2
    exit 2
}
server_ipv4=${server_address%:*}
server_port=${server_address##*:}
((10#$server_port >= 1 && 10#$server_port <= 65535)) || {
    echo "server port is outside 1..65535" >&2
    exit 2
}
IFS=. read -r -a server_octets <<<"$server_ipv4"
for octet in "${server_octets[@]}"; do
    ((10#$octet <= 255)) || {
        echo "invalid IPv4 server address" >&2
        exit 2
    }
done
command -v openssl >/dev/null 2>&1 || {
    echo "openssl is required to generate the PSK" >&2
    exit 1
}

install -d -m 0700 "$repo_dir/.canis"
if [[ -e "$repo_dir/.canis/psk" && $force -ne 1 ]]; then
    echo ".canis/psk already exists; use --force to replace all generated material" >&2
    exit 1
fi
umask 077
openssl rand 32 >"$repo_dir/.canis/psk"
chmod 0600 "$repo_dir/.canis/psk"

sed "s|^psk_file = .*|psk_file = \"$repo_dir/.canis/psk\"|" \
    "$repo_dir/examples/server.toml" >"$repo_dir/.canis/server.toml"
sed "s|^server_addr = .*|server_addr = \"$server_address\"|" \
    "$repo_dir/examples/client.toml" >"$repo_dir/.canis/client.toml"
chmod 0600 "$repo_dir/.canis/server.toml" "$repo_dir/.canis/client.toml"

echo "generated .canis/psk, .canis/server.toml, and .canis/client.toml"
