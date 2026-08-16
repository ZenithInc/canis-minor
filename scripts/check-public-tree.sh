#!/usr/bin/env bash
set -euo pipefail

for forbidden in .canis server.txt target website/.astro website/dist website/node_modules; do
    if git ls-files --error-unmatch "$forbidden" >/dev/null 2>&1 || git ls-files "$forbidden/" | grep -q .; then
        echo "forbidden generated or private path is tracked: $forbidden" >&2
        exit 1
    fi
done

if git ls-files | grep -E '(^|/)(psk|known_hosts)$' >/dev/null; then
    echo "a PSK or known_hosts file is tracked" >&2
    exit 1
fi

if git grep -nE '85[.]190[.]102[.]5|111[.]62[.]156[.]213' -- ':!scripts/check-public-tree.sh'; then
    echo "a known private infrastructure address is present in the public tree" >&2
    exit 1
fi

if git grep -nE -- '-----BEGIN (RSA |EC |DSA |OPENSSH )?PRIVATE KEY-----'; then
    echo "a private key header is present in the public tree" >&2
    exit 1
fi

echo "public tree excludes generated state, deployment secrets, and known private addresses"
