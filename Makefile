.PHONY: build install run check fmt

build:
	cargo build --release

install: build
	cp target/release/chloe-pied /usr/local/bin/chloe-pied

run: install
	chloe-pied

check:
	cargo clippy -- -D warnings

fmt:
	cargo fmt
