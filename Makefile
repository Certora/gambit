.PHONY: linux
linux:
	cargo build --release --target x86_64-unknown-linux-gnu
	cargo install --path .
	cargo clippy
	cargo fmt
	cargo test --release
	python3 scripts/sanity_check.py

.PHONY: macos
macos:
	rustup target add aarch64-apple-darwin
	cargo build --release --target=x86_64-apple-darwin
	cargo build --release --target=aarch64-apple-darwin
	cargo install --path .
	cargo clippy
	cargo fmt