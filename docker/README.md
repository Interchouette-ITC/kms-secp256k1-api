# Docker image (kms-secp256k1-api)

Size-optimized multi-stage build → `gcr.io/distroless/cc-debian13:nonroot` (Debian 13 / trixie family).

| Item | Value |
| --- | --- |
| Binary | `kms-secp256k1-api` |
| Builder | `rust:slim-trixie` |
| Port | `APP_PORT` (4000 prod / 4001 test typical) |
| Compose | `docker-compose.prod.yml` / `docker-compose.test.yml` / `docker-compose.mcp.yml` |

## Slim MCP sidecar image

Slim Rust image (`mcp/Dockerfile`) with `docker` CLI + compose plugin so Make lifecycle tools work against the mounted repo.

| Registry | Image |
| --- | --- |
| Docker Hub | `interchouette/kms-secp256k1-api-mcp` |
| Personal GHCR | `ghcr.io/groussac/kms-secp256k1-api-mcp` |
| Org GHCR | `ghcr.io/interchouette-itc/kms-secp256k1-api-mcp` |

```bash
make mcp-docker-build        # :latest + :$(MCP_VERSION)
make mcp-docker-build-dev    # :dev (Hub + GHCR tags)
make mcp-http                # pull Hub image (or build) → Streamable HTTP :7790
make mcp-http-stop
make mcp-docker-push-dev     # local interactive logins (:dev)
make mcp-docker-push-release # local interactive logins (:version + :latest)
# CI: “CI/CD MCP Image dev” → :dev; “CI/CD MCP Image release tags” + GitHub Release → :version/:latest
```

Docs: [`docs/mcp.md`](../docs/mcp.md).

## Where to pull images

| Registry | Image |
| --- | --- |
| Docker Hub | `interchouette/kms-secp256k1-api` |
| GHCR | `ghcr.io/interchouette-itc/kms-secp256k1-api` |

```bash
docker pull interchouette/kms-secp256k1-api:dev
docker pull ghcr.io/interchouette-itc/kms-secp256k1-api:dev
```

## Tags

| Tag | Who pushes | When |
| --- | --- | --- |
| `:dev` | Local `make` or Actions `workflow_dispatch` (“CI/CD Image dev”) | On demand |
| `:X.Y.Z` | GitHub Actions on **Release** | Tag `vX.Y.Z` must match `Cargo.toml` |
| `:latest` | Same release workflow | Moves with each release |

## Release images and binary

| Piece | Status |
| --- | --- |
| CI (check, lint, test, rustdoc pages) | Live on `dev` |
| `:dev` image push (Hub + GHCR) | Live via “CI/CD Image dev” |
| Versioned release images `:X.Y.Z` + `:latest` | Cut with GitHub Release tag `vX.Y.Z` (= `Cargo.toml`) |
| Release binary (`kms-secp256k1-api`) | Attached on that Release (embeds WASM; optional `WASM_PATH` / on-disk `./wasm/wasm.wasm`) |

Current version **1.2.0** (see [`CHANGELOG.md`](../docs/CHANGELOG.md)); publish `:X.Y.Z` + `:latest` by creating GitHub Release tag `v1.2.0`.

To cut a release:

1. `make version-show` (or bump with `make version-bump-patch` etc.)
2. Merge version / changelog to `dev` if needed
3. Create a GitHub Release with tag **`v$(APP_VERSION)`** (must equal `Cargo.toml`)
4. Workflow pushes Hub + GHCR tags and attaches the Linux binary

## Local build / push `:dev`

```bash
make docker-build-dev
make docker-push-dev
```

## Other make targets

```bash
make docker-build
make docker-run
make docker-run-test
make docker-stop
make docker-build-no-cache
make docker-inspect
make version-show
```
