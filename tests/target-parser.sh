#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
source "$repo_dir/scripts/target-lib.sh"
temp_dir=$(mktemp -d)
trap 'rm -rf -- "$temp_dir"' EXIT

printf '%s\n' 'ssh -p 22222 deploy@192.0.2.8' >"$temp_dir/good"
parse_ssh_target "$temp_dir/good"
[[ $SSH_DESTINATION == deploy@192.0.2.8 && $SSH_PORT -eq 22222 ]]

printf '%s\n' 'ssh user@192.0.2.8;touch /tmp/pwned' >"$temp_dir/injection"
if parse_ssh_target "$temp_dir/injection" 2>/dev/null; then
    echo "parser accepted shell syntax" >&2
    exit 1
fi

printf '%s\n' 'ssh -p 70000 user@192.0.2.8' >"$temp_dir/bad-port"
if parse_ssh_target "$temp_dir/bad-port" 2>/dev/null; then
    echo "parser accepted an invalid port" >&2
    exit 1
fi

echo "SSH target parser tests passed"
