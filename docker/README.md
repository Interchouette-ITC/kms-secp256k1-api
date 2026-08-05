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

**Host binds:** product compose (prod/test/localstack) has **no** host data volumes today. The MCP sidecar still mounts the clone at `/workspace` and uses host `docker.sock`. Always pass `KMS_HOST_ROOT` to the absolute host clone (`make mcp-http` does). Lifecycle start tools refuse when `KMS_API_ROOT=/workspace` without a safe `KMS_HOST_ROOT`. See [`docs/mcp.md`](../docs/mcp.md).

| Registry | Image |
| --- | --- |
| Docker Hub | `interchouette/kms-secp256k1-api-mcp` |
| Personal GHCR | `ghcr.io/groussac/kms-secp256k1-api-mcp` |
| Org GHCR | `ghcr.io/interchouette-itc/kms-secp256k1-api-mcp` |

```bash
make mcp-docker-build        # :latest + :$(MCP_VERSION)
make mcp-docker-build-dev    # :dev
make mcp-http                # pull Hub image (or build) → Streamable HTTP :7790
make mcp-http-stop
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

| Tag | Meaning |
| --- | --- |
| `:dev` | Rolling development image |
| `:X.Y.Z` | Versioned release (matches `Cargo.toml` / GitHub Release `vX.Y.Z`) |
| `:latest` | Latest release |

Release assets also include the Linux binary on the GitHub Release. See [`CHANGELOG.md`](../docs/CHANGELOG.md) for the current version.

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
```
