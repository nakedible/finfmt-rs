.PHONY: fmt check bench

fmt:
	cargo +nightly fmt

# Linking the benches is what runs the no-panic checks: they call every
# asm wrapper, and a library build alone never links.
check:
	cargo +nightly fmt --check
	cargo clippy --all-features --all-targets -- -D warnings
	cargo clippy --no-default-features --lib -- -D warnings
	cargo test --all-features
	cargo test --no-default-features --lib
	RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features
	cargo build --release --all-features --benches

bench:
	cargo bench --features asm-inspect
