all: build test sanity

.PHONY: build
build:
	cargo build --release
	cargo install --path .
	cargo clippy
	cargo fmt

.PHONY: test
test:
	cargo test --release

.PHONY: sanity
sanity:
	python3 scripts/sanity_check.py
