# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

First org-cut release will be **1.1.0** (matches current `Cargo.toml`). Notes below describe the 1.1.0-era tree; cut the dated section when tagging `v1.1.0`.

### API

- HTTP API for secp256k1 key create / sign / verify / list / delete
- Blockchain modes: Ethereum, Cosmos, Casper
- AWS KMS backend for production key material (testing mode for local mocks)
- Typed domain errors via `thiserror` (`KmsError` / `crate::Result`) through services, KMS client, WASM loader, and crypto helpers; HTTP maps variants via `KmsError::status` and `Display` into JSON


### Docker

- Distroless `cc-debian13` runtime; builder `rust:slim-trixie`
- API image on Hub `interchouette/kms-secp256k1-api`, personal GHCR `ghcr.io/groussac/kms-secp256k1-api`, org GHCR `ghcr.io/interchouette-itc/kms-secp256k1-api`
- LocalStack image `interchouette/kms-localstack` (+ `ghcr.io/groussac/kms-localstack`, `ghcr.io/interchouette-itc/kms-localstack`)
- `:dev` via Actions (API: CI/CD Image dev; LocalStack: CI/CD LocalStack Image dev); `:X.Y.Z` + `:latest` via GitHub Release for both images
- Optional Cargo chain features: default `casper`; `make` / CI / Docker use `--features all`

### Testing

- Dual backends: mock (`make test`) and LocalStack KMS (`make test-localstack`)

### Docs and tooling

- Keep a Changelog + `make version-show` / version bump targets
- Release workflow attaches the Linux `kms-secp256k1-api` binary and publishes API + LocalStack to Hub and both GHCR registries
