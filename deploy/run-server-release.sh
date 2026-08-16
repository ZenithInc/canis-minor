#!/usr/bin/env bash
set -euo pipefail

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
config=${1:-$script_dir/server.toml}
exec "$script_dir/canis-server" --config "$config"
