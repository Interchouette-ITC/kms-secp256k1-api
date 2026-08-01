# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.2.0] - 2026-08-01

### Changed

- Full `Cargo.toml` dependency floor refresh to current crates.io max-stable for every direct dep (`toml` 1.1.4, `axum-extra` 0.12, `base64` 0.23, AWS SDK floors, `utoipa` 5.5, `serial_test` 4, pinned minors across the board)
- Lockfile refresh
- Maintenance release

### Notes

- `k256` remains on 0.13.4 (cosmrs / ethers / casper-types require `^0.13`; 0.14 is max-stable but not graph-compatible here)

## [1.1.0] - 2026-08-01

First public release.

### Added

- HTTP API for secp256k1 key create, sign, verify, list, and delete
- Blockchain modes: Ethereum, Cosmos, and Casper
- AWS KMS backend for production key material, with a local testing mode
- Standalone Linux binary (WASM crypto module included; no sidecar file required)
- Optional on-disk WASM override via `./wasm/wasm.wasm` or `WASM_PATH`
- Docker images for the API and LocalStack-backed testing
- OpenAPI / Swagger UI

### Notes

- Typed domain errors (`KmsError`) mapped to HTTP status and JSON responses
- Optional Cargo features select chain support (`casper` default; `all` for full image builds)

[Unreleased]: https://github.com/Interchouette-ITC/kms-secp256k1-api/compare/v1.2.0...HEAD
[1.2.0]: https://github.com/Interchouette-ITC/kms-secp256k1-api/releases/tag/v1.2.0
[1.1.0]: https://github.com/Interchouette-ITC/kms-secp256k1-api/releases/tag/v1.1.0
