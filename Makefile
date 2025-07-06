build:
	cargo build

test:
	cargo test

lint:
	cargo clippy --bins -- -D warnings -D clippy::all -D clippy::pedantic -D clippy::nursery

check-lint:
	cargo clippy --fix --allow-dirty --allow-staged -- -D warnings -D clippy::all -D clippy::pedantic -D clippy::nursery
