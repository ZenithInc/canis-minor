#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
target_file=${1:-$repo_dir/server.txt}
source "$repo_dir/scripts/target-lib.sh"
parse_ssh_target "$target_file"

binary="$repo_dir/target/x86_64-unknown-linux-musl/release/canis-client"
for required in "$binary" "$repo_dir/.canis/client.toml" "$repo_dir/.canis/psk"; do
    [[ -f $required ]] || {
        echo "missing deployment artifact: $required" >&2
        exit 1
    }
done
[[ $(wc -c <"$repo_dir/.canis/psk") -eq 32 ]] || {
    echo "PSK must contain exactly 32 bytes" >&2
    exit 1
}

known_hosts="$repo_dir/.canis/known_hosts"
ssh_args=(-p "$SSH_PORT" -o BatchMode=yes -o "UserKnownHostsFile=$known_hosts" -o StrictHostKeyChecking=accept-new -- "$SSH_DESTINATION")
scp_args=(-P "$SSH_PORT" -o BatchMode=yes -o "UserKnownHostsFile=$known_hosts" -o StrictHostKeyChecking=accept-new)
remote_dir=$(ssh "${ssh_args[@]}" 'mktemp -d /tmp/canis-minor.XXXXXX')
[[ $remote_dir =~ ^/tmp/canis-minor\.[A-Za-z0-9]+$ ]] || {
    echo "remote host returned an unsafe temporary path" >&2
    exit 1
}
cleanup_upload() {
    # shellcheck disable=SC2029 # remote_dir is validated against a strict safe pattern above.
    ssh "${ssh_args[@]}" "rm -rf -- '$remote_dir'" >/dev/null 2>&1 || true
}
trap cleanup_upload EXIT

scp "${scp_args[@]}" \
    "$binary" \
    "$repo_dir/.canis/client.toml" \
    "$repo_dir/.canis/psk" \
    "$repo_dir/deploy/canis-client.service" \
    "$repo_dir/deploy/remote-install.sh" \
    "$SSH_DESTINATION:$remote_dir/"

# shellcheck disable=SC2029 # remote_dir is validated and intentionally expanded locally.
ssh "${ssh_args[@]}" \
    "if [ \"\$(id -u)\" -eq 0 ]; then bash '$remote_dir/remote-install.sh' '$remote_dir'; else sudo bash '$remote_dir/remote-install.sh' '$remote_dir'; fi"
trap - EXIT
