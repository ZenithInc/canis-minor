## Summary

<!-- What changed and why? -->

## Impact and boundaries

<!-- Describe user/operator impact, security implications, and traffic/platform boundaries. -->

## Validation

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo test --workspace`
- [ ] `bash tests/target-parser.sh`
- [ ] Website checks run when documentation or site files changed
- [ ] Real TPROXY acceptance evidence included when privileged networking behavior changed

## Checklist

- [ ] I added or updated tests for behavior changes.
- [ ] I updated both English and Simplified Chinese user documentation where needed.
- [ ] I did not include a PSK, SSH target, known-hosts data, real infrastructure address, or generated build output.
- [ ] I reviewed security and fail-open cleanup implications.
- [ ] I agree that my contribution is licensed under `MIT OR Apache-2.0`.
