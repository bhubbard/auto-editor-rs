.PHONY: all build test clean run check

all: build

build:
	cargo build --release

test:
	cargo test

check:
	cargo check
	cargo clippy

clean:
	cargo clean
