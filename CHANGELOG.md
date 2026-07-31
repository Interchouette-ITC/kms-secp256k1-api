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
- Typed domain errors via `thiserror` (`KmsError` / `crate::Result`) at service and KMS client trait boundaries; HTTP maps `Display` into JSON

### Docker

- Distroless `cc-debian13` runtime; builder `rust:slim-trixie`
- Hub image `interchouette/kms-secp256k1-api`; GHCR `ghcr.io/interchouette-itc/kms-secp256k1-api`
- `:dev` via Actions workflow dispatch; `:X.Y.Z` + `:latest` via GitHub Release

### Docs and tooling

- Keep a Changelog + `make version-show` / version bump targets
- Release workflow attaches the Linux `kms-secp256k1-api` binary
