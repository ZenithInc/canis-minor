#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
target=x86_64-unknown-linux-musl

command -v rustup >/dev/null 2>&1 || {
    echo "rustup is required" >&2
    exit 1
}
rustup target add "$target"
cargo build --manifest-path "$repo_dir/Cargo.toml" \
    --locked --release --target "$target" -p canis-client -p canis-server

for component in canis-client canis-server; do
    binary="$repo_dir/target/$target/release/$component"
    test -x "$binary"
    if command -v file >/dev/null 2>&1; then
        file "$binary"
    fi
    echo "static binary: $binary"
done
