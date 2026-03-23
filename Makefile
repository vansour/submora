.PHONY: check test clippy clippy-wasm serve build release-check

check:
	cargo fmt --all -- --check
	cargo check --workspace
	cargo check -p submora-web --target wasm32-unknown-unknown
	cargo test --workspace

test:
	cargo test --workspace

clippy:
	cargo clippy --workspace --all-targets -- -D warnings

clippy-wasm:
	cargo clippy -p submora-web --target wasm32-unknown-unknown -- -D warnings

serve:
	dx serve --platform web --package submora-web

build:
	dx build --platform web --package submora-web --release

release-check: check clippy clippy-wasm
