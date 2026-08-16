#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
config=${1:-$repo_dir/.canis/server.toml}
exec "$repo_dir/target/release/canis-server" --config "$config"
