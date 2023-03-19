all_linux: linux test sanity
all_macos: macos test sanity

.PHONY: linux
linux:
	cargo build --release --target x86_64-unknown-linux-gnu
	cargo install --path . --force
	cargo clippy
	cargo fmt

.PHONY: macos
macos:
	rustup target add aarch64-apple-darwin
	cargo build --release --target=x86_64-apple-darwin
	cargo build --release --target=aarch64-apple-darwin
	cargo install --path . --force
	cargo clippy
	cargo fmt

.PHONY: test
test:
	cargo test --release

.PHONY: sanity
sanity:
	python3 scripts/sanity_check.py