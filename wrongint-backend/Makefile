.PHONY: ci
ci: test-verbose clippy fmt-check

.PHONY: cleanup
cleanup: fix clippy fmt

.PHONY: test
test:
	cargo test

.PHONY: test-verbose
test-verbose:
	RUST_BACKTRACE=1 cargo test -- --nocapture

.PHONY: clippy
clippy:
	cargo clippy --all-targets -- -D warnings

.PHONY: fmt
fmt:
	cargo fmt

.PHONY: fmt-check
fmt-check:
	cargo fmt --check

.PHONY: fix
fix:
	cargo fix --allow-staged --allow-dirty --all-targets
	cargo clippy --fix --allow-staged --allow-dirty --all-targets
	cargo fmt

.PHONY: local
local:
	cargo run -- run local_config.toml
