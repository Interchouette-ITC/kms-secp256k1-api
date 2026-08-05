# kms-secp256k1-api

AWS KMS-backed HTTP API for secp256k1 key operations (Ethereum, Cosmos, Casper) and Docker deployment.

Canonical repo: [Interchouette-ITC/kms-secp256k1-api](https://github.com/Interchouette-ITC/kms-secp256k1-api).

## Docs

| Doc | Description |
| --- | --- |
| [`OVERVIEW.md`](OVERVIEW.md) | Product overview, API, Docker, env |
| [`Tutorial.md`](Tutorial.md) | Tutorial |
| [`mcp.md`](mcp.md) | MCP sidecar (Make/LocalStack + HTTP tools) |
| [`CHANGELOG.md`](CHANGELOG.md) | Semver notes |
| [`docker/README.md`](../docker/README.md) | Image tags and registries |
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

See [`mcp.md`](mcp.md) and [`mcp/README.md`](../mcp/README.md).

Swagger UI (when running): `http://localhost:<APP_PORT>/docs/`

## Docker images

| Registry | Image |
| --- | --- |
| Docker Hub | `interchouette/kms-secp256k1-api`, `interchouette/kms-localstack`, `interchouette/kms-secp256k1-api-mcp` |
| Personal GHCR | `ghcr.io/groussac/kms-secp256k1-api`, `…/kms-localstack`, `…/kms-secp256k1-api-mcp` |
| Org GHCR | `ghcr.io/interchouette-itc/kms-secp256k1-api`, `…/kms-localstack`, `…/kms-secp256k1-api-mcp` |

```bash
docker pull interchouette/kms-secp256k1-api:dev
make docker-build-dev && make docker-push-dev
```

Details: [`docker/README.md`](../docker/README.md).

Not published to crates.io.

## License

MIT. See [`LICENSE`](../LICENSE).
