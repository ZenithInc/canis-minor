#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cargo build --manifest-path "$repo_dir/Cargo.toml" --locked --release -p canis-server
echo "server binary: $repo_dir/target/release/canis-server"
