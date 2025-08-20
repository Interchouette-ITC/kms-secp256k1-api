build:
	cargo build

test:
	cargo test -- --nocapture

lint:
	cargo clippy --bins -- -D warnings -D clippy::all -D clippy::pedantic -D clippy::nursery

check-lint:
	cargo clippy --fix --allow-dirty --allow-staged -- -D warnings -D clippy::all -D clippy::pedantic -D clippy::nursery

APP_NAME=kms-secp256k1-api
TAG=latest

docker-build:
	docker build --network=host -t $(APP_NAME):$(TAG) -f ./docker/Dockerfile .

docker-build-no-cache:
	docker build --network=host --no-cache -t $(APP_NAME):$(TAG) -f ./docker/Dockerfile .

docker-run-test:
	docker compose -f ./docker/docker-compose.test.yml up --no-build --force-recreate

docker-run:
	docker compose -f ./docker/docker-compose.prod.yml up -d --force-recreate

docker-stop:
	docker compose -f ./docker/docker-compose.prod.yml stop

doc:
	cargo doc --package kms-secp256k1-api --no-deps
	cp -r target/doc/* docs/api-rust/
