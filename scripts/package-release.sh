#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
tag=${1:?usage: package-release.sh vX.Y.Z [OUTPUT_DIRECTORY]}
output_dir=${2:-$repo_dir/dist}
target=x86_64-unknown-linux-musl

[[ $tag =~ ^v[0-9]+[.][0-9]+[.][0-9]+$ ]] || {
    echo "release tag must match vX.Y.Z" >&2
    exit 2
}
version=${tag#v}
workspace_version=$(awk '
    /^\[workspace.package\]$/ { in_package = 1; next }
    /^\[/ { in_package = 0 }
    in_package && $1 == "version" { gsub(/"/, "", $3); print $3; exit }
' "$repo_dir/Cargo.toml")
[[ $workspace_version == "$version" ]] || {
    echo "tag $tag does not match Cargo workspace version $workspace_version" >&2
    exit 1
}
[[ -n $output_dir && $output_dir != / ]] || {
    echo "unsafe output directory" >&2
    exit 1
}
for component in canis-client canis-server; do
    [[ -x $repo_dir/target/$target/release/$component ]] || {
        echo "missing release binary: $component" >&2
        exit 1
    }
done
[[ ${SOURCE_DATE_EPOCH:-0} =~ ^[0-9]+$ ]] || {
    echo "SOURCE_DATE_EPOCH must be an integer" >&2
    exit 2
}

install -d "$output_dir"
client_archive="canis-client-$tag-$target.tar.gz"
server_archive="canis-server-$tag-$target.tar.gz"
rm -f -- "$output_dir/$client_archive" "$output_dir/$server_archive" "$output_dir/SHA256SUMS"

staging=$(mktemp -d)
trap 'rm -rf -- "$staging"' EXIT
client_root="canis-client-$tag-$target"
server_root="canis-server-$tag-$target"
install -d "$staging/$client_root" "$staging/$server_root"

install -m 0755 "$repo_dir/target/$target/release/canis-client" "$staging/$client_root/canis-client"
install -m 0644 "$repo_dir/examples/client.toml" "$staging/$client_root/client.toml.example"
install -m 0644 "$repo_dir/deploy/canis-client.service" "$staging/$client_root/canis-client.service"
install -m 0755 "$repo_dir/deploy/install-client.sh" "$staging/$client_root/install-client.sh"
install -m 0755 "$repo_dir/deploy/remote-uninstall.sh" "$staging/$client_root/uninstall-client.sh"
install -m 0644 "$repo_dir/release/client-README.md" "$staging/$client_root/README.md"

install -m 0755 "$repo_dir/target/$target/release/canis-server" "$staging/$server_root/canis-server"
install -m 0644 "$repo_dir/examples/server.toml" "$staging/$server_root/server.toml.example"
install -m 0755 "$repo_dir/deploy/run-server-release.sh" "$staging/$server_root/run-server.sh"
install -m 0644 "$repo_dir/release/server-README.md" "$staging/$server_root/README.md"

for package_root in "$client_root" "$server_root"; do
    install -m 0644 "$repo_dir/LICENSE" "$staging/$package_root/LICENSE"
    install -m 0644 "$repo_dir/LICENSE-MIT" "$staging/$package_root/LICENSE-MIT"
    install -m 0644 "$repo_dir/LICENSE-APACHE" "$staging/$package_root/LICENSE-APACHE"
done

archive() {
    local root=$1 destination=$2
    tar --sort=name --owner=0 --group=0 --numeric-owner \
        --mtime="@${SOURCE_DATE_EPOCH:-0}" -C "$staging" -cf - "$root" |
        gzip -n >"$destination"
}
archive "$client_root" "$output_dir/$client_archive"
archive "$server_root" "$output_dir/$server_archive"
(
    cd -- "$output_dir"
    sha256sum "$client_archive" "$server_archive" >SHA256SUMS
)
echo "release artifacts written to $output_dir"
