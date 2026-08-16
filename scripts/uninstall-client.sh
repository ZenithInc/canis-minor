#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
target_file=${1:-$repo_dir/server.txt}
source "$repo_dir/scripts/target-lib.sh"
parse_ssh_target "$target_file"

known_hosts="$repo_dir/.canis/known_hosts"
ssh_args=(-p "$SSH_PORT" -o BatchMode=yes -o "UserKnownHostsFile=$known_hosts" -o StrictHostKeyChecking=accept-new -- "$SSH_DESTINATION")
remote_path=/tmp/canis-minor-uninstall.sh
scp -P "$SSH_PORT" -o BatchMode=yes -o "UserKnownHostsFile=$known_hosts" -o StrictHostKeyChecking=accept-new \
    "$repo_dir/deploy/remote-uninstall.sh" "$SSH_DESTINATION:$remote_path"
# shellcheck disable=SC2029 # remote_path is a fixed literal and intentionally expanded locally.
ssh "${ssh_args[@]}" \
    "if [ \"\$(id -u)\" -eq 0 ]; then bash '$remote_path'; else sudo bash '$remote_path'; fi; rm -f -- '$remote_path'"
