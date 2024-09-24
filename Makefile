.PHONY: pre-commit
pre-commit: fmt clippy test

.PHONY: build
build:
	@ cargo build --workspace

.PHONY: clean
clean:
	@ cargo clean
	@ git clean -fdx

.PHONY: clippy
clippy:
	@ cargo clippy --workspace -- -D warnings

.PHONY: fmt
fmt:
	@ cargo +nightly fmt --all

.PHONY: test
test:
	@ cargo test --workspace
