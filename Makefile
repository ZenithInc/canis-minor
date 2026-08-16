.PHONY: check test build-server build-static verify-static init deploy uninstall

check:
	cargo fmt --all -- --check
	cargo clippy --workspace --all-targets -- -D warnings

test:
	cargo test --workspace
	bash tests/target-parser.sh

build-server:
	bash scripts/build-server.sh

build-static:
	bash scripts/build-static.sh

verify-static:
	bash scripts/check-static.sh

init:
	bash scripts/init.sh

deploy:
	bash scripts/deploy-client.sh

uninstall:
	bash scripts/uninstall-client.sh
