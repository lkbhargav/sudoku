.PHONY: run
run:
	cargo run --release

.PHONY: build
build:
	cargo build --release

.PHONY: runb
runb:
	. ./target/release/sudoku
