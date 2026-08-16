# Canis Minor server release

This archive contains the Linux x86_64 static PIE server for the release named by the parent directory.

1. Verify the archive against the release `SHA256SUMS` and GitHub attestation before extraction.
2. Copy `server.toml.example` to `server.toml`, set `psk_file` to a private file containing the client's same 32 raw PSK bytes, and review `dns_upstream`.
3. Permit only the configured TCP listener through the host/provider firewall.
4. Run in the foreground for initial validation:

   ```bash
   RUST_LOG=info ./run-server.sh ./server.toml
   ```

Use your own service supervisor and a dedicated working directory for production. The server creates no nftables or policy-routing state.

Full documentation: https://zenithinc.github.io/canis-minor/
