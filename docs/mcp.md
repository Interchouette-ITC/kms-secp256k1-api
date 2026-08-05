# MCP for agents

Rust binary `kms-secp256k1-api-mcp` **v1.2.1** (mcpkit), dual transport (stdio / Streamable HTTP).

**Separate package:** lives in [`mcp/`](../mcp/) — **no dependency** on the `kms-secp256k1-api` library crate. Lifecycle uses Make/Docker; product tools call the HTTP API.

Published image: [`interchouette/kms-secp256k1-api-mcp`](https://hub.docker.com/r/interchouette/kms-secp256k1-api-mcp) (`:1.2.1`, `:latest`, `:dev`).

## Run without compiling

```bash
# MCP HTTP sidecar (needs Docker socket + this repo mounted as workspace)
docker pull interchouette/kms-secp256k1-api-mcp:1.2.1
docker run --rm -d --name kms-secp256k1-api-mcp \
  -p 7790:7790 \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -v "$PWD":/workspace \
  -e KMS_API_ROOT=/workspace \
  -e KMS_HOST_ROOT="$PWD" \
  interchouette/kms-secp256k1-api-mcp:1.2.1
```

`KMS_HOST_ROOT` is **required** in the MCP container: absolute host clone path (never `/workspace`, never `/`). Lifecycle **start** tools refuse without it.

From a clone, `make mcp-http` pulls the Hub image (builds locally only if pull fails) and passes `KMS_HOST_ROOT=$(CURDIR)`.

| Mode | How | Endpoint |
| --- | --- | --- |
| **HTTP** (Docker / Hub) | `make mcp-http` | `http://127.0.0.1:7790/mcp` |
| **HTTP** (host) | `make run-mcp-http` | same URL |
| **stdio** (host) | `make run-mcp` | process on stdin/stdout |

```bash
kms-secp256k1-api-mcp                         # stdio
kms-secp256k1-api-mcp --http                  # 127.0.0.1:7790
kms-secp256k1-api-mcp --http --listen 0.0.0.0:7790
```

Example Cursor config: [`mcp/mcp.json.example`](../mcp/mcp.json.example).

## Make matrix (MCP itself)

| Command | Effect |
| --- | --- |
| `make mcp-build` | Host release-build MCP binary |
| `make mcp-docker-build` | Build `kms-secp256k1-api-mcp:1.2.1` (+ `:latest`) |
| `make mcp-docker-build-dev` | Build `:dev` (Hub + GHCR tags) |
| `make mcp-http` | Pull Hub image (or build) + start sidecar on **7790** |
| `make mcp-http-stop` | Stop MCP sidecar |
| `make mcp-docker-push-dev` / `mcp-docker-push-release` | Push Hub + GHCR |
| `make run-mcp` | Host **stdio** MCP |
| `make run-mcp-http` | Host HTTP on `127.0.0.1:7790` |

Compose: [`docker/docker-compose.mcp.yml`](../docker/docker-compose.mcp.yml). Image Dockerfile: [`mcp/Dockerfile`](../mcp/Dockerfile) (includes `docker`/`make`/`curl` so lifecycle tools work via the mounted repo + docker.sock).

### Host binds / docker.sock (read this)

| Claim | Verdict |
| --- | --- |
| Lifecycle tools emit host `/workspace` data binds today | **False** — product compose (prod/test/localstack) has **no** data `volumes:` |
| MCP uses sock + in-container `/workspace` | **True** — setup only |
| Same multi-GB `/` blowup as NCTL today | **No** |

The MCP sidecar is started from the **host** (`make mcp-http` / ensure scripts) with `docker-compose.mcp.yml` (`${KMS_HOST_ROOT}:/workspace` + `docker.sock`). MCP lifecycle tools **never** call that compose file; they drive product compose only.

If someone later adds host binds to product compose using container `/workspace` or `$PWD` while talking to the host daemon, Docker would create **host** `/workspace` on the root disk. Guard:

- Always pass `-e KMS_HOST_ROOT=<absolute host clone>`
- Start tools (`kms_docker_run`, `kms_docker_run_test`, `kms_docker_run_localstack`, `kms_stack_start`) **refuse** when `KMS_API_ROOT=/workspace` and `KMS_HOST_ROOT` is missing or unsafe
- Regression tests lock product compose free of host binds and ensure MCP src never references `docker-compose.mcp.yml`

**Security limit:** mounting `docker.sock` is still host-root equivalent for anything Docker can do. The refuse guard only blocks the known `/workspace` bind-root class. Rootless Docker is **not** a substitute for `KMS_HOST_ROOT`.

### Published MCP images

| Registry | Image |
| --- | --- |
| Docker Hub | `interchouette/kms-secp256k1-api-mcp` |
| Personal GHCR | `ghcr.io/groussac/kms-secp256k1-api-mcp` |
| Org GHCR | `ghcr.io/interchouette-itc/kms-secp256k1-api-mcp` |

Tags: `:dev` (CI on `mcp/**` / workflow_dispatch), `:X.Y.Z` + `:latest` on `mcp/**` push to `dev` and on GitHub Release (same cadence as API/LocalStack).

### Tests

```bash
cd mcp
cargo test                         # unit + FakeRepo + HTTP/stdio transport + compose regress
cargo test --test api_roundtrip    # host mock create/list/delete
KMS_MCP_LIVE=1 cargo test --test api_roundtrip -- --ignored --nocapture  # LocalStack stack
```

## Tool catalog

### Lifecycle (Make parity)

MCP drives **Docker/Make** from `KMS_API_ROOT`. Not exposed: `docker-push-*`, `version-bump-*`, `version-set`.

| Tool | Make / behavior |
| --- | --- |
| `kms_help` | `make help` + tool map |
| `kms_build` / `kms_build_release` / `kms_check` | matching targets; optional `features` |
| `kms_lint` / `kms_test` / `kms_verify` | mock suite |
| `kms_test_localstack` | `make test-localstack` (long) |
| `kms_docker_build` / `_dev` / `_no_cache` | API images |
| `kms_docker_build_localstack` / `_dev` | LocalStack images |
| `kms_docker_run` | prod compose `:4000` (refuses unsafe host bind root) |
| `kms_docker_run_test` | test compose **detached** `:4001` (Make is foreground) |
| `kms_docker_stop` / `kms_docker_stop_all` | stop prod (+ tear down test) |
| `kms_docker_run_localstack` / `kms_docker_stop_localstack` | LocalStack `:4566` |
| `kms_docker_inspect` / `kms_version_show` | inspect / version |
| `kms_stack_start` / `kms_stack_stop` | test-localstack compose (API `:4001` + LocalStack) |
| `kms_api_start` / `kms_api_stop` | host release binary mock API (pid under `mcp/.run/`) |
| `kms_status` | containers + `GET /` on `:4000` and `:4001` |

### HTTP API

Base URL: `KMS_API_URL` (default `http://127.0.0.1:4000`).

| Tool | Route |
| --- | --- |
| `kms_hello` | `GET /` |
| `kms_create_key` | `POST /createKey` |
| `kms_sign_transaction_hash` | `POST /signTransactionHash` |
| `kms_sign_transaction` | `POST /signTransaction` |
| `kms_verify_signature` | `GET /verifySignature` |
| `kms_delete_key` | `DELETE /deleteKey` |
| `kms_list_keys` | `GET /listKeys` |
| `kms_openapi` | `GET /docs/openapi.json` summary |

## Examples (humans / CI)

```bash
export KMS_API_ROOT=$PWD
export KMS_HOST_ROOT=$PWD
cargo run --manifest-path mcp/Cargo.toml --example help
cargo run --manifest-path mcp/Cargo.toml --example status
cargo run --manifest-path mcp/Cargo.toml --example roundtrip   # host mock API
```

See [`mcp/examples/README.md`](../mcp/examples/README.md).
