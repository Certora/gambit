all: build test

.PHONY: build
build:
	cargo build --release
	cargo clippy
	cargo fmt

.PHONY: test
test:
	cargo test --release