# kms-secp256k1-api

AWS KMS-backed HTTP API for secp256k1 key operations (Ethereum, Cosmos, Casper) and Docker deployment.

Canonical repo: [Interchouette-ITC/kms-secp256k1-api](https://github.com/Interchouette-ITC/kms-secp256k1-api).

## Docs

| Doc | Description |
| --- | --- |
| [`docs/README.md`](docs/README.md) | Product overview, API, Docker, env |
| [`docs/Tutorial.md`](docs/Tutorial.md) | Tutorial |
| [`docs/mcp.md`](docs/mcp.md) | MCP sidecar (Make/LocalStack + HTTP tools) |
| [`CHANGELOG.md`](CHANGELOG.md) | Semver notes |
| [`docker/README.md`](docker/README.md) | Image tags and release playbook |
| `make doc` → rustdoc | [GitHub Pages](https://interchouette-itc.github.io/kms-secp256k1-api/kms_secp256k1_api) |

## Quick start

```bash
make build
make test
make docker-build
make docker-run-test
```

MCP (agents / Cursor):

```bash
make run-mcp          # stdio
make run-mcp-http     # http://127.0.0.1:7790/mcp (host)
make mcp-http         # same URL via Docker sidecar image
```

See [`docs/mcp.md`](docs/mcp.md) and [`mcp/README.md`](mcp/README.md).

Swagger UI (when running): `http://localhost:<APP_PORT>/docs/`

## Docker images

| Registry | Image |
| --- | --- |
| Docker Hub | `interchouette/kms-secp256k1-api`, `interchouette/kms-localstack`, `interchouette/kms-secp256k1-api-mcp` |
| Personal GHCR | `ghcr.io/groussac/kms-secp256k1-api`, `…/kms-localstack`, `…/kms-secp256k1-api-mcp` |
| Org GHCR | `ghcr.io/interchouette-itc/kms-secp256k1-api`, `…/kms-localstack`, `…/kms-secp256k1-api-mcp` |

```bash
docker pull interchouette/kms-secp256k1-api:dev
make docker-build-dev && make docker-push-dev   # :dev when you want (local)
# GitHub Actions → "CI/CD Image dev" (workflow_dispatch) for :dev
# GitHub Release tag vX.Y.Z → pushes :X.Y.Z and :latest (+ binary)
make version-show
```

Details: [`docker/README.md`](docker/README.md).

## Release playbook

1. `make version-show` (or `make version-bump-patch` / `version-set VERSION=x.y.z`)
2. Update [`CHANGELOG.md`](CHANGELOG.md); merge to `dev`
3. Create a GitHub Release on the org repo with tag **`v$(APP_VERSION)`** (must equal `Cargo.toml`)
4. Creating the GitHub Release publishes Docker images (`:version` / `:latest`) and attaches the Linux binary (standalone; optional `WASM_PATH` / on-disk `./wasm/wasm.wasm` override)

Not published to crates.io.

## License

MIT. See [`LICENSE`](LICENSE).
