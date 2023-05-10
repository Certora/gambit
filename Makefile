all_linux: linux test regression
all_macos: macos test regression

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

regression:
	bash scripts/run_regressions.sh