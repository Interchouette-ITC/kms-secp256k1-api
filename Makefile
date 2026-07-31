# kms-secp256k1-api - developer targets

APP_NAME ?= kms-secp256k1-api
HUB_IMAGE ?= interchouette/kms-secp256k1-api
GHCR_PERSONAL_IMAGE ?= ghcr.io/groussac/kms-secp256k1-api
GHCR_ORG_IMAGE ?= ghcr.io/interchouette-itc/kms-secp256k1-api
TAG ?= latest
APP_VERSION ?= $(shell awk '/^version = /{gsub(/"/, "", $$3); print $$3; exit}' Cargo.toml)
DOCKERFILE ?= docker/Dockerfile
DOCKER_BUILDKIT ?= 1
CI ?= 0
COMPOSE_PROD ?= docker/docker-compose.prod.yml
COMPOSE_TEST ?= docker/docker-compose.test.yml

.DEFAULT_GOAL := help

.PHONY: help build build-release check test \
	lint format clippy check-lint doc \
	docker-build docker-build-no-cache \
	docker-build-dev docker-push-dev \
	docker-push-dev-hub docker-push-dev-ghcr-personal docker-push-dev-ghcr-itc \
	docker-push-release docker-push-release-hub \
	docker-push-release-ghcr-personal docker-push-release-ghcr-itc \
	docker-hub-description \
	docker-run docker-run-test docker-stop docker-inspect \
	version-show version-bump-patch version-bump-minor version-bump-major version-set

help:
	@echo "kms-secp256k1-api targets"
	@echo ""
	@echo "  make build / build-release / check / test / lint"
	@echo "  make docker-build          Build $(HUB_IMAGE):$(TAG) (+ :$(APP_VERSION))"
	@echo "  make docker-build-dev      Build and tag :dev (Hub + GHCR names)"
	@echo "  make docker-push-dev       Push :dev (local interactive logins)"
	@echo "  make docker-hub-description  Sync Hub short + full description"
	@echo "  make docker-push-release   Tag/push release images (CI uses split targets)"
	@echo "  make docker-run / docker-run-test / docker-stop"
	@echo "  make version-show          Print Cargo.toml version + suggested tag"
	@echo "  make version-bump-patch|minor|major"
	@echo "  make version-set VERSION=x.y.z"
	@echo ""
	@echo "Release: make version-show → GitHub Release tag v\$$(APP_VERSION)"
	@echo "Overrides: HUB_IMAGE=$(HUB_IMAGE) APP_VERSION=$(APP_VERSION) CI=0|1 TAG=$(TAG)"

build:
	cargo build

build-release:
	cargo build --release

check:
	cargo check --all --locked

test:
	cargo test -- --nocapture

format:
	cargo fmt

clippy:
	cargo clippy --bins -- -D warnings -D clippy::all -D clippy::pedantic -D clippy::nursery

lint: format clippy

check-lint: format
	cargo clippy --fix --allow-dirty --allow-staged -- -D warnings -D clippy::all -D clippy::pedantic -D clippy::nursery

doc:
	cargo doc --package kms-secp256k1-api --no-deps
	cp -r target/doc/* docs/api-rust/

# ---------------------------------------------------------------------------
# Docker
# ---------------------------------------------------------------------------

docker-build:
	DOCKER_BUILDKIT=$(DOCKER_BUILDKIT) docker build --pull --network=host \
		-t $(APP_NAME):$(TAG) \
		-t $(HUB_IMAGE):$(TAG) \
		-t $(HUB_IMAGE):$(APP_VERSION) \
		-f $(DOCKERFILE) \
		.

docker-build-no-cache:
	DOCKER_BUILDKIT=$(DOCKER_BUILDKIT) docker build --pull --network=host --no-cache \
		-t $(APP_NAME):$(TAG) \
		-t $(HUB_IMAGE):$(TAG) \
		-t $(HUB_IMAGE):$(APP_VERSION) \
		-f $(DOCKERFILE) \
		.

docker-build-dev:
	DOCKER_BUILDKIT=$(DOCKER_BUILDKIT) docker build --pull --network=host \
		-t $(APP_NAME):dev \
		-t $(HUB_IMAGE):dev \
		-t $(GHCR_PERSONAL_IMAGE):dev \
		-t $(GHCR_ORG_IMAGE):dev \
		-f $(DOCKERFILE) \
		.

docker-hub-description:
	python3 docker/sync-hub-description.py

docker-push-dev-hub:
	docker push $(HUB_IMAGE):dev
	$(MAKE) docker-hub-description

docker-push-dev-ghcr-personal:
	docker push $(GHCR_PERSONAL_IMAGE):dev

docker-push-dev-ghcr-itc:
	docker push $(GHCR_ORG_IMAGE):dev

docker-push-dev:
	@if [ "$(CI)" = "1" ]; then \
		echo "Use docker-push-dev-hub / docker-push-dev-ghcr-personal / docker-push-dev-ghcr-itc in CI"; \
		exit 1; \
	fi
	@echo "Logging in to Docker Hub..."; \
	docker login || { echo "Docker Hub login failed"; exit 1; }
	$(MAKE) docker-push-dev-hub
	@echo "Logging in to GHCR (personal)..."; \
	docker login ghcr.io || { echo "Skipping personal GHCR"; exit 0; }
	$(MAKE) docker-push-dev-ghcr-personal
	@echo "Logging in to GHCR (org)..."; \
	docker login ghcr.io || { echo "Skipping org GHCR"; exit 0; }
	$(MAKE) docker-push-dev-ghcr-itc

docker-push-release-hub:
	docker push $(HUB_IMAGE):$(APP_VERSION)
	docker push $(HUB_IMAGE):latest
	$(MAKE) docker-hub-description

docker-push-release-ghcr-personal:
	docker tag $(HUB_IMAGE):$(APP_VERSION) $(GHCR_PERSONAL_IMAGE):$(APP_VERSION)
	docker tag $(HUB_IMAGE):latest $(GHCR_PERSONAL_IMAGE):latest
	docker push $(GHCR_PERSONAL_IMAGE):$(APP_VERSION)
	docker push $(GHCR_PERSONAL_IMAGE):latest

docker-push-release-ghcr-itc:
	docker tag $(HUB_IMAGE):$(APP_VERSION) $(GHCR_ORG_IMAGE):$(APP_VERSION)
	docker tag $(HUB_IMAGE):latest $(GHCR_ORG_IMAGE):latest
	docker push $(GHCR_ORG_IMAGE):$(APP_VERSION)
	docker push $(GHCR_ORG_IMAGE):latest

docker-push-release: docker-push-release-hub docker-push-release-ghcr-personal docker-push-release-ghcr-itc

docker-run-test:
	docker compose -f $(COMPOSE_TEST) up --no-build --force-recreate

docker-run:
	docker compose -f $(COMPOSE_PROD) up -d --force-recreate

docker-stop:
	docker compose -f $(COMPOSE_PROD) stop

docker-inspect:
	@docker image inspect $(HUB_IMAGE):$(TAG) --format \
		'{{.RepoTags}} size={{.Size}} created={{.Created}}' 2>/dev/null \
		|| docker image inspect $(HUB_IMAGE):dev --format \
			'{{.RepoTags}} size={{.Size}} created={{.Created}}' 2>/dev/null \
		|| echo "Image not found - run make docker-build or make docker-build-dev"

# ---------------------------------------------------------------------------
# Version (Cargo.toml); release images via GitHub Release
# ---------------------------------------------------------------------------

version-show:
	@echo "Current version: $(APP_VERSION)"; \
	echo ""; \
	echo "Suggested GitHub Release tag:"; \
	echo "  v$(APP_VERSION)"; \
	echo ""; \
	echo "When creating a GitHub Release, use the Tag field (not only the title)."

version-bump-patch:
	@current="$(APP_VERSION)"; \
	new=$$(echo "$$current" | awk -F. '{print $$1"."$$2"."($$3+1)}'); \
	sed -i "s/^version = \"$$current\"/version = \"$$new\"/" Cargo.toml; \
	echo "Version bumped from $$current to $$new"

version-bump-minor:
	@current="$(APP_VERSION)"; \
	new=$$(echo "$$current" | awk -F. '{print $$1"."($$2+1)".0"}'); \
	sed -i "s/^version = \"$$current\"/version = \"$$new\"/" Cargo.toml; \
	echo "Version bumped from $$current to $$new"

version-bump-major:
	@current="$(APP_VERSION)"; \
	new=$$(echo "$$current" | awk -F. '{print ($$1+1)".0.0"}'); \
	sed -i "s/^version = \"$$current\"/version = \"$$new\"/" Cargo.toml; \
	echo "Version bumped from $$current to $$new"

version-set:
	@if [ -z "$(VERSION)" ]; then \
		echo "Usage: make version-set VERSION=x.y.z"; \
		exit 1; \
	fi; \
	current="$(APP_VERSION)"; \
	sed -i "s/^version = \"$$current\"/version = \"$(VERSION)\"/" Cargo.toml; \
	echo "Version set from $$current to $(VERSION)"
