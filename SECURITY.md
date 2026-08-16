# Security policy

## Supported versions

| Version | Supported |
| --- | --- |
| Latest 0.1.x release | Yes |
| Older releases | No |
| Unreleased `main` | Best effort |

## Report a vulnerability

Do not open a public issue or discussion for an undisclosed vulnerability. Use GitHub's private vulnerability reporting form at:

https://github.com/ZenithInc/canis-minor/security/advisories/new

Include the affected version and platform, impact, reproduction steps or proof of concept, relevant logs with secrets and infrastructure details removed, and any proposed mitigation. Never include a live PSK.

You should receive an acknowledgement within 7 days. We aim to provide an initial assessment within 14 days and will coordinate disclosure and remediation timing with the reporter. These targets may vary with complexity and maintainer availability.

## Scope

Reports about the Noise/PSK handshake, stream framing, transparent interception, route cleanup, privilege boundaries, deployment scripts, release provenance, and dependency supply chain are in scope. General support requests, scanner output without demonstrated impact, denial of service requiring control of the host, and behavior explicitly outside the documented traffic boundary are normally handled through Discussions instead.

## Operational security

Canis Minor uses one shared 32-byte PSK and does not yet provide per-client identity or automated rotation. A holder of the PSK can authenticate to the tunnel. The server can reach destinations available from its network. Operators must protect and rotate the PSK, restrict the server with host/network firewalls, keep an out-of-band recovery path, and verify release checksums and attestations.
