# Canis Minor client release

This archive contains the Linux x86_64 static PIE client for the release named by the parent directory.

1. Verify the archive against the release `SHA256SUMS` and GitHub attestation before extraction.
2. Copy `client.toml.example` to a private `client.toml` and replace the RFC 5737 server address.
3. Create or securely receive the same 32-byte raw PSK used by the server: `umask 077; openssl rand 32 > psk`.
4. Keep an independent management session and provider console available.
5. Review the scripts and install:

   ```bash
   sudo ./install-client.sh --config ./client.toml --psk ./psk
   ```

Uninstall with `sudo ./uninstall-client.sh`. Cleanup must run before manually deleting configuration or the binary.

Full documentation: https://zenithinc.github.io/canis-minor/
