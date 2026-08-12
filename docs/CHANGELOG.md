# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- MCP sidecar migrates from mcpkit to rmcp 3.1.2 (stdio + Streamable HTTP)
- Dependency floors: `thiserror` 2.0.20, `base64` 0.23.1, `async-trait` 0.1.92, `http-body-util` 0.1.5; MCP `reqwest` 0.13
- `k256` remains on 0.13.4 (cosmrs / ethers / casper-types require `^0.13`)

## [1.2.1] - 2026-08-05

### Security

- MCP: require `KMS_HOST_ROOT` (absolute host clone) when `KMS_API_ROOT=/workspace`; lifecycle start tools refuse unsafe bind roots that could create host `/workspace` on `/`. Product compose has no host data binds today - this closes the latent sock+/workspace path. See [`docs/mcp.md`](docs/mcp.md).

### Added

- Compose regression tests (product compose stay bind-free; MCP ops never call `docker-compose.mcp.yml`)
- `mcp/mcp.json.example` with `KMS_HOST_ROOT`

## [1.2.0] - 2026-08-02

### Added

- MCP sidecar package [`mcp/`](mcp/) (`kms-secp256k1-api-mcp`): Make/Docker lifecycle + HTTP API tools (stdio / Streamable HTTP on `:7790`); Docker image published to Hub + GHCR (`make mcp-docker-push-*`); transport + create/delete tests; see [`docs/mcp.md`](docs/mcp.md)

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

### Changed

- Dependency floor refresh to current crates.io max-stable for direct deps (`toml` 1.1.4, `axum-extra` 0.12, `base64` 0.23, AWS SDK floors, `utoipa` 5.5, `serial_test` 4, pinned minors)
- Lockfile refresh

### Notes

- Typed domain errors (`KmsError`) mapped to HTTP status and JSON responses
- Optional Cargo features select chain support (`casper` default; `all` for full image builds)
- `k256` remains on 0.13.4 (cosmrs / ethers / casper-types require `^0.13`)

[Unreleased]: https://github.com/Interchouette-ITC/kms-secp256k1-api/compare/v1.2.1...HEAD
[1.2.1]: https://github.com/Interchouette-ITC/kms-secp256k1-api/releases/tag/v1.2.1
[1.2.0]: https://github.com/Interchouette-ITC/kms-secp256k1-api/releases/tag/v1.2.0
[1.1.0]: https://github.com/Interchouette-ITC/kms-secp256k1-api/releases/tag/v1.1.0
