# kms-secp256k1-api - developer targets

APP_NAME ?= kms-secp256k1-api
HUB_IMAGE ?= interchouette/kms-secp256k1-api
GHCR_PERSONAL_IMAGE ?= ghcr.io/groussac/kms-secp256k1-api
GHCR_WORKER_IMAGE ?= ghcr.io/interchouette/kms-secp256k1-api
GHCR_ORG_IMAGE ?= ghcr.io/interchouette-itc/kms-secp256k1-api
LOCALSTACK_NAME ?= kms-localstack
LOCALSTACK_HUB_IMAGE ?= interchouette/kms-localstack
LOCALSTACK_GHCR_PERSONAL_IMAGE ?= ghcr.io/groussac/kms-localstack
LOCALSTACK_GHCR_WORKER_IMAGE ?= ghcr.io/interchouette/kms-localstack
LOCALSTACK_GHCR_ORG_IMAGE ?= ghcr.io/interchouette-itc/kms-localstack
MCP_NAME ?= kms-secp256k1-api-mcp
MCP_HUB_IMAGE ?= interchouette/kms-secp256k1-api-mcp
MCP_GHCR_PERSONAL_IMAGE ?= ghcr.io/groussac/kms-secp256k1-api-mcp
MCP_GHCR_WORKER_IMAGE ?= ghcr.io/interchouette/kms-secp256k1-api-mcp
MCP_GHCR_ORG_IMAGE ?= ghcr.io/interchouette-itc/kms-secp256k1-api-mcp
TAG ?= latest
APP_VERSION ?= $(shell awk '/^version = /{gsub(/"/, "", $$3); print $$3; exit}' Cargo.toml)
MCP_VERSION ?= $(shell awk '/^version = /{gsub(/"/, "", $$3); print $$3; exit}' mcp/Cargo.toml)
DOCKERFILE ?= docker/Dockerfile
DOCKERFILE_LOCALSTACK ?= docker/Dockerfile-localstack
DOCKER_BUILDKIT ?= 1
CI ?= 0
COMPOSE_PROD ?= docker/docker-compose.prod.yml
COMPOSE_TEST ?= docker/docker-compose.test.yml
COMPOSE_LOCALSTACK ?= docker/docker-compose.localstack.yml
COMPOSE_TEST_LOCALSTACK ?= docker/docker-compose.test-localstack.yml
COMPOSE_MCP ?= docker/docker-compose.mcp.yml

# Chain Cargo features: casper | ethereum | cosmos | all
# Default all so make check/test/docker stay full-coverage.
# Fast local: make build FEATURES=casper  (or plain `cargo build` = casper only)
FEATURES ?= all
CARGO_FEATURES := --no-default-features --features $(FEATURES)

.DEFAULT_GOAL := help

.PHONY: help build build-release check test test-localstack verify \
	lint format format-check clippy check-lint doc \
	docker-build docker-build-no-cache \
	docker-build-dev docker-push-dev \
	docker-push-dev-hub docker-push-dev-ghcr-personal docker-push-dev-ghcr-itc \
	docker-push-release docker-push-release-hub \
	docker-push-release-ghcr-personal docker-push-release-ghcr-itc \
	docker-build-localstack docker-build-localstack-dev \
	docker-push-localstack-dev-hub docker-push-localstack-dev-ghcr-personal \
	docker-push-localstack-dev-ghcr-itc docker-push-localstack-dev \
	docker-push-localstack-release-hub docker-push-localstack-release-ghcr-personal \
	docker-push-localstack-release-ghcr-itc docker-push-localstack-release \
	docker-run docker-run-test docker-run-localstack docker-stop-localstack \
	docker-stop docker-inspect \
	mcp-build mcp-docker-build mcp-docker-build-dev mcp-http mcp-http-stop \
	run-mcp run-mcp-http \
	mcp-docker-push-dev mcp-docker-push-dev-hub \
	mcp-docker-push-dev-ghcr-personal mcp-docker-push-dev-ghcr-itc \
	mcp-docker-push-release mcp-docker-push-release-hub \
	mcp-docker-push-release-ghcr-personal mcp-docker-push-release-ghcr-itc \
	version-show version-bump-patch version-bump-minor version-bump-major version-set

CLIPPY_FLAGS := -D warnings -D clippy::all -D clippy::pedantic -D clippy::nursery

help:
	@echo "kms-secp256k1-api targets"
	@echo ""
	@echo "  make build / build-release / check / test / lint / verify"
	@echo "  make test-localstack       Integration tests against LocalStack KMS"
	@echo "  make doc                   rustdoc → docs/api-rust/ (commit with source)"
	@echo "  make docker-build          Build $(HUB_IMAGE):$(TAG) (+ :$(APP_VERSION))"
	@echo "  make docker-build-dev      Build and tag :dev (Hub + GHCR names)"
	@echo "  make docker-build-localstack  Build $(LOCALSTACK_HUB_IMAGE):$(TAG)"
	@echo "  make docker-push-dev       Push :dev (local interactive logins)"
	@echo "  make docker-push-release   Tag/push release images (CI uses split targets)"
	@echo "  make docker-run / docker-run-test / docker-run-localstack / docker-stop"
	@echo "  make mcp-build / mcp-docker-build / mcp-docker-build-dev / mcp-http"
	@echo "  make mcp-docker-push-dev / mcp-docker-push-release"
	@echo "  make run-mcp / run-mcp-http"
	@echo "  make version-show          Print Cargo.toml version + suggested tag"
	@echo "  make version-bump-patch|minor|major"
	@echo "  make version-set VERSION=x.y.z"
	@echo ""
	@echo "Features (FEATURES=$(FEATURES)):"
	@echo "  casper | ethereum | cosmos | all"
	@echo "  make build FEATURES=casper   # fast single-chain"
	@echo "  cargo build                  # default feature = casper only"
	@echo "  make test / make check       # --features all (full suite)"
	@echo "  Docker images always build with --features all"
	@echo ""
	@echo "Release: make version-show → GitHub Release tag v\$$(APP_VERSION)"
	@echo "Overrides: HUB_IMAGE=$(HUB_IMAGE) APP_VERSION=$(APP_VERSION) CI=0|1 TAG=$(TAG) FEATURES=$(FEATURES)"

build:
	cargo build $(CARGO_FEATURES)

build-release:
	cargo build --release $(CARGO_FEATURES)

check:
	cargo check --all --locked $(CARGO_FEATURES)

test: lint
	KMS_TEST_BACKEND=mock cargo test $(CARGO_FEATURES) -- --nocapture

# Integration tests against LocalStack (requires Docker). Builds image if missing.
test-localstack: lint docker-build-localstack
	@set -e; \
	docker compose -f $(COMPOSE_LOCALSTACK) up -d --force-recreate; \
	trap 'docker compose -f $(COMPOSE_LOCALSTACK) down -v --remove-orphans' EXIT; \
	echo "Waiting for LocalStack health..."; \
	for i in $$(seq 1 60); do \
		status=$$(docker inspect --format='{{.State.Health.Status}}' kms-localstack 2>/dev/null || echo starting); \
		if [ "$$status" = "healthy" ]; then break; fi; \
		if [ "$$i" -eq 60 ]; then echo "LocalStack did not become healthy"; exit 1; fi; \
		sleep 2; \
	done; \
	KMS_TEST_BACKEND=localstack \
	AWS_ENDPOINT=http://127.0.0.1:4566 \
	AWS_REGION=eu-west-3 \
	cargo test $(CARGO_FEATURES) --test mod -- --nocapture

format:
	cargo fmt

format-check:
	cargo fmt -- --check

clippy:
	cargo clippy --all-targets $(CARGO_FEATURES) -- $(CLIPPY_FLAGS)

lint: format-check clippy

check-lint: format-check
	cargo clippy --fix --allow-dirty --allow-staged --all-targets $(CARGO_FEATURES) -- $(CLIPPY_FLAGS)

verify: format-check clippy test
	@echo "verify OK"

# Prefer CARGO_TARGET_DIR when set (CI/sandbox); else repo-local target/doc.
DOC_OUT ?= $(if $(CARGO_TARGET_DIR),$(CARGO_TARGET_DIR)/doc,target/doc)

doc:
	RUSTDOCFLAGS='-D warnings' cargo doc --package kms-secp256k1-api --no-deps $(CARGO_FEATURES)
	@test -d "$(DOC_OUT)/kms_secp256k1_api" || (echo "missing $(DOC_OUT)/kms_secp256k1_api"; exit 1)
	@rm -rf docs/api-rust
	@mkdir -p docs/api-rust
	@cp -a "$(DOC_OUT)/." docs/api-rust/
	@printf '%s\n' \
		'<!DOCTYPE html>' \
		'<html lang="en">' \
		'<head>' \
		'<meta charset="utf-8">' \
		'<meta http-equiv="refresh" content="0; url=kms_secp256k1_api/index.html">' \
		'<title>kms-secp256k1-api</title>' \
		'<link rel="canonical" href="kms_secp256k1_api/index.html">' \
		'</head>' \
		'<body><p><a href="kms_secp256k1_api/index.html">kms-secp256k1-api rustdoc</a></p></body>' \
		'</html>' \
		> docs/api-rust/index.html
	@rm -f docs/api-rust/README.md
	@echo "docs/api-rust/ updated - open docs/api-rust/kms_secp256k1_api/index.html"

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
		-t $(GHCR_WORKER_IMAGE):dev \
		-t $(GHCR_ORG_IMAGE):dev \
		-f $(DOCKERFILE) \
		.

docker-push-dev-hub:
	docker push $(HUB_IMAGE):dev

docker-push-dev-ghcr-personal:
	docker push $(GHCR_PERSONAL_IMAGE):dev

docker-push-dev-ghcr-itc:
	docker push $(GHCR_WORKER_IMAGE):dev
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

docker-push-release-ghcr-personal:
	docker tag $(HUB_IMAGE):$(APP_VERSION) $(GHCR_PERSONAL_IMAGE):$(APP_VERSION)
	docker tag $(HUB_IMAGE):latest $(GHCR_PERSONAL_IMAGE):latest
	docker push $(GHCR_PERSONAL_IMAGE):$(APP_VERSION)
	docker push $(GHCR_PERSONAL_IMAGE):latest

docker-push-release-ghcr-itc:
	docker tag $(HUB_IMAGE):$(APP_VERSION) $(GHCR_WORKER_IMAGE):$(APP_VERSION)
	docker tag $(HUB_IMAGE):latest $(GHCR_WORKER_IMAGE):latest
	docker tag $(HUB_IMAGE):$(APP_VERSION) $(GHCR_ORG_IMAGE):$(APP_VERSION)
	docker tag $(HUB_IMAGE):latest $(GHCR_ORG_IMAGE):latest
	docker push $(GHCR_WORKER_IMAGE):$(APP_VERSION)
	docker push $(GHCR_WORKER_IMAGE):latest
	docker push $(GHCR_ORG_IMAGE):$(APP_VERSION)
	docker push $(GHCR_ORG_IMAGE):latest

docker-push-release: docker-push-release-hub docker-push-release-ghcr-personal docker-push-release-ghcr-itc

docker-run-test:
	docker compose -f $(COMPOSE_TEST) up --no-build --force-recreate

docker-run-localstack:
	$(MAKE) docker-build-localstack
	docker compose -f $(COMPOSE_LOCALSTACK) up -d --force-recreate

docker-stop-localstack:
	docker compose -f $(COMPOSE_LOCALSTACK) down -v --remove-orphans

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
# LocalStack image
# ---------------------------------------------------------------------------

docker-build-localstack:
	DOCKER_BUILDKIT=$(DOCKER_BUILDKIT) docker build --pull --network=host \
		-t $(LOCALSTACK_NAME):$(TAG) \
		-t $(LOCALSTACK_HUB_IMAGE):$(TAG) \
		-t $(LOCALSTACK_HUB_IMAGE):$(APP_VERSION) \
		-f $(DOCKERFILE_LOCALSTACK) \
		docker

docker-build-localstack-dev:
	DOCKER_BUILDKIT=$(DOCKER_BUILDKIT) docker build --pull --network=host \
		-t $(LOCALSTACK_NAME):dev \
		-t $(LOCALSTACK_HUB_IMAGE):dev \
		-t $(LOCALSTACK_GHCR_PERSONAL_IMAGE):dev \
		-t $(LOCALSTACK_GHCR_WORKER_IMAGE):dev \
		-t $(LOCALSTACK_GHCR_ORG_IMAGE):dev \
		-f $(DOCKERFILE_LOCALSTACK) \
		docker

docker-push-localstack-dev-hub:
	docker push $(LOCALSTACK_HUB_IMAGE):dev

docker-push-localstack-dev-ghcr-personal:
	docker push $(LOCALSTACK_GHCR_PERSONAL_IMAGE):dev

docker-push-localstack-dev-ghcr-itc:
	docker push $(LOCALSTACK_GHCR_WORKER_IMAGE):dev
	docker push $(LOCALSTACK_GHCR_ORG_IMAGE):dev

docker-push-localstack-dev:
	@if [ "$(CI)" = "1" ]; then \
		echo "Use docker-push-localstack-dev-hub / docker-push-localstack-dev-ghcr-* in CI"; \
		exit 1; \
	fi
	@echo "Logging in to Docker Hub..."; \
	docker login || { echo "Docker Hub login failed"; exit 1; }
	$(MAKE) docker-push-localstack-dev-hub
	@echo "Logging in to GHCR (personal)..."; \
	docker login ghcr.io || { echo "Skipping personal GHCR"; exit 0; }
	$(MAKE) docker-push-localstack-dev-ghcr-personal
	@echo "Logging in to GHCR (org)..."; \
	docker login ghcr.io || { echo "Skipping org GHCR"; exit 0; }
	$(MAKE) docker-push-localstack-dev-ghcr-itc

docker-push-localstack-release-hub:
	docker push $(LOCALSTACK_HUB_IMAGE):$(APP_VERSION)
	docker push $(LOCALSTACK_HUB_IMAGE):latest

docker-push-localstack-release-ghcr-personal:
	docker tag $(LOCALSTACK_HUB_IMAGE):$(APP_VERSION) $(LOCALSTACK_GHCR_PERSONAL_IMAGE):$(APP_VERSION)
	docker tag $(LOCALSTACK_HUB_IMAGE):latest $(LOCALSTACK_GHCR_PERSONAL_IMAGE):latest
	docker push $(LOCALSTACK_GHCR_PERSONAL_IMAGE):$(APP_VERSION)
	docker push $(LOCALSTACK_GHCR_PERSONAL_IMAGE):latest

docker-push-localstack-release-ghcr-itc:
	docker tag $(LOCALSTACK_HUB_IMAGE):$(APP_VERSION) $(LOCALSTACK_GHCR_WORKER_IMAGE):$(APP_VERSION)
	docker tag $(LOCALSTACK_HUB_IMAGE):latest $(LOCALSTACK_GHCR_WORKER_IMAGE):latest
	docker tag $(LOCALSTACK_HUB_IMAGE):$(APP_VERSION) $(LOCALSTACK_GHCR_ORG_IMAGE):$(APP_VERSION)
	docker tag $(LOCALSTACK_HUB_IMAGE):latest $(LOCALSTACK_GHCR_ORG_IMAGE):latest
	docker push $(LOCALSTACK_GHCR_WORKER_IMAGE):$(APP_VERSION)
	docker push $(LOCALSTACK_GHCR_WORKER_IMAGE):latest
	docker push $(LOCALSTACK_GHCR_ORG_IMAGE):$(APP_VERSION)
	docker push $(LOCALSTACK_GHCR_ORG_IMAGE):latest

docker-push-localstack-release: docker-push-localstack-release-hub \
	docker-push-localstack-release-ghcr-personal docker-push-localstack-release-ghcr-itc

# ---------------------------------------------------------------------------
# MCP sidecar (mcp/ — separate Cargo package; no dep on API lib)
# ---------------------------------------------------------------------------

mcp-build:
	cargo build --manifest-path mcp/Cargo.toml --release

mcp-docker-build:
	DOCKER_BUILDKIT=$(DOCKER_BUILDKIT) docker build --network=host \
		-t $(MCP_NAME):$(TAG) \
		-t $(MCP_HUB_IMAGE):$(TAG) \
		-t $(MCP_HUB_IMAGE):$(MCP_VERSION) \
		-f mcp/Dockerfile \
		mcp

mcp-docker-build-dev:
	DOCKER_BUILDKIT=$(DOCKER_BUILDKIT) docker build --network=host \
		-t $(MCP_NAME):dev \
		-t $(MCP_HUB_IMAGE):dev \
		-t $(MCP_GHCR_PERSONAL_IMAGE):dev \
		-t $(MCP_GHCR_WORKER_IMAGE):dev \
		-t $(MCP_GHCR_ORG_IMAGE):dev \
		-f mcp/Dockerfile \
		mcp

mcp-docker-push-dev-hub:
	docker push $(MCP_HUB_IMAGE):dev

mcp-docker-push-dev-ghcr-personal:
	docker push $(MCP_GHCR_PERSONAL_IMAGE):dev

mcp-docker-push-dev-ghcr-itc:
	docker push $(MCP_GHCR_WORKER_IMAGE):dev
	docker push $(MCP_GHCR_ORG_IMAGE):dev

mcp-docker-push-dev:
	@if [ "$(CI)" = "1" ]; then \
		echo "Use mcp-docker-push-dev-hub / mcp-docker-push-dev-ghcr-* in CI"; \
		exit 1; \
	fi
	@echo "Logging in to Docker Hub..."; \
	docker login || { echo "Docker Hub login failed"; exit 1; }
	$(MAKE) mcp-docker-push-dev-hub
	@echo "Logging in to GHCR (personal)..."; \
	docker login ghcr.io || { echo "Skipping personal GHCR"; exit 0; }
	$(MAKE) mcp-docker-push-dev-ghcr-personal
	@echo "Logging in to GHCR (org)..."; \
	docker login ghcr.io || { echo "Skipping org GHCR"; exit 0; }
	$(MAKE) mcp-docker-push-dev-ghcr-itc

mcp-docker-push-release-hub:
	docker push $(MCP_HUB_IMAGE):$(MCP_VERSION)
	docker push $(MCP_HUB_IMAGE):latest

mcp-docker-push-release-ghcr-personal:
	docker tag $(MCP_HUB_IMAGE):$(MCP_VERSION) $(MCP_GHCR_PERSONAL_IMAGE):$(MCP_VERSION)
	docker tag $(MCP_HUB_IMAGE):latest $(MCP_GHCR_PERSONAL_IMAGE):latest
	docker push $(MCP_GHCR_PERSONAL_IMAGE):$(MCP_VERSION)
	docker push $(MCP_GHCR_PERSONAL_IMAGE):latest

mcp-docker-push-release-ghcr-itc:
	docker tag $(MCP_HUB_IMAGE):$(MCP_VERSION) $(MCP_GHCR_WORKER_IMAGE):$(MCP_VERSION)
	docker tag $(MCP_HUB_IMAGE):latest $(MCP_GHCR_WORKER_IMAGE):latest
	docker tag $(MCP_HUB_IMAGE):$(MCP_VERSION) $(MCP_GHCR_ORG_IMAGE):$(MCP_VERSION)
	docker tag $(MCP_HUB_IMAGE):latest $(MCP_GHCR_ORG_IMAGE):latest
	docker push $(MCP_GHCR_WORKER_IMAGE):$(MCP_VERSION)
	docker push $(MCP_GHCR_WORKER_IMAGE):latest
	docker push $(MCP_GHCR_ORG_IMAGE):$(MCP_VERSION)
	docker push $(MCP_GHCR_ORG_IMAGE):latest

mcp-docker-push-release: mcp-docker-push-release-hub \
	mcp-docker-push-release-ghcr-personal mcp-docker-push-release-ghcr-itc

# Prefer Hub image (no local compile). Falls back to local build if pull fails.
mcp-http:
	-docker pull $(MCP_HUB_IMAGE):$(MCP_VERSION)
	@if ! docker image inspect $(MCP_HUB_IMAGE):$(MCP_VERSION) >/dev/null 2>&1 \
		&& ! docker image inspect $(MCP_NAME):$(MCP_VERSION) >/dev/null 2>&1; then \
		echo "Hub image missing; building locally…"; \
		$(MAKE) mcp-docker-build; \
	fi
	KMS_MCP_IMAGE=$(MCP_HUB_IMAGE):$(MCP_VERSION) KMS_HOST_ROOT="$(CURDIR)" \
		docker compose -f $(COMPOSE_MCP) up -d --force-recreate

mcp-http-stop:
	-KMS_HOST_ROOT="$(CURDIR)" docker compose -f $(COMPOSE_MCP) down --remove-orphans
	-docker stop kms-secp256k1-api-mcp 2>/dev/null
	-docker rm kms-secp256k1-api-mcp 2>/dev/null

run-mcp:
	KMS_API_ROOT="$(CURDIR)" KMS_HOST_ROOT="$(CURDIR)" cargo run --manifest-path mcp/Cargo.toml --quiet --

run-mcp-http:
	KMS_API_ROOT="$(CURDIR)" KMS_HOST_ROOT="$(CURDIR)" cargo run --manifest-path mcp/Cargo.toml --quiet -- \
		--http --listen 127.0.0.1:7790

# ---------------------------------------------------------------------------
# Version (Cargo.toml); release images via GitHub Release
# ---------------------------------------------------------------------------

version-show:
	@echo "Current version: $(APP_VERSION)"; \
	echo "MCP package:     $$(awk '/^version = /{gsub(/"/, "", $$3); print $$3; exit}' mcp/Cargo.toml)"; \
	echo ""; \
	echo "Suggested GitHub Release tag:"; \
	echo "  v$(APP_VERSION)"; \
	echo ""; \
	echo "When creating a GitHub Release, use the Tag field (not only the title)."

# Bump root + mcp Cargo.toml, mcp_server attr, then refresh lockfiles (required for --locked CI).
define version-apply
	@current="$(APP_VERSION)"; \
	new="$(1)"; \
	if [ -z "$$new" ]; then echo "empty version"; exit 1; fi; \
	sed -i "s/^version = \"$$current\"/version = \"$$new\"/" Cargo.toml; \
	sed -i "s/^version = \"$$current\"/version = \"$$new\"/" mcp/Cargo.toml; \
	sed -i 's/\(#\[mcp_server(name = "kms-secp256k1-api", version = "\)[^"]*\("\]\)/\1'$$new'\2/' mcp/src/server.rs; \
	cargo metadata --format-version 1 --no-deps >/dev/null; \
	cargo metadata --manifest-path mcp/Cargo.toml --format-version 1 --no-deps >/dev/null; \
	cargo check --all --locked --no-default-features --features all -q; \
	cargo check --manifest-path mcp/Cargo.toml --locked -q; \
	echo "Version $$current → $$new (tomls + mcp_server + both Cargo.lock)"
endef

version-bump-patch:
	$(call version-apply,$(shell echo "$(APP_VERSION)" | awk -F. '{print $$1"."$$2"."($$3+1)}'))

version-bump-minor:
	$(call version-apply,$(shell echo "$(APP_VERSION)" | awk -F. '{print $$1"."($$2+1)".0"}'))

version-bump-major:
	$(call version-apply,$(shell echo "$(APP_VERSION)" | awk -F. '{print ($$1+1)".0.0"}'))

version-set:
	@if [ -z "$(VERSION)" ]; then \
		echo "Usage: make version-set VERSION=x.y.z"; \
		exit 1; \
	fi
	$(call version-apply,$(VERSION))
