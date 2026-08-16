#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
target=x86_64-unknown-linux-musl

for component in canis-client canis-server; do
    binary="$repo_dir/target/$target/release/$component"
    [[ -x $binary ]] || {
        echo "missing executable: $binary" >&2
        exit 1
    }

    description=$(file --brief "$binary")
    [[ $description == *"ELF 64-bit LSB pie executable"* && $description == *"x86-64"* && $description == *"static-pie linked"* ]] || {
        echo "$component is not an x86_64 static PIE: $description" >&2
        exit 1
    }
    if readelf -d "$binary" | grep -q '(NEEDED)'; then
        echo "$component declares a dynamic dependency" >&2
        exit 1
    fi
    "$binary" --help >/dev/null
    "$binary" --version >/dev/null
    echo "$component: verified x86_64 static PIE with no dynamic dependencies"
done
