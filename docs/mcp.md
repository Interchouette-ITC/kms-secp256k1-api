# MCP for agents

Rust binary `kms-secp256k1-api-mcp` **v1.2.0** (mcpkit), dual transport like casper-nctl-2-docker / tvscreener.

**Separate package:** lives in [`mcp/`](../mcp/) — **no dependency** on the `kms-secp256k1-api` library crate. Lifecycle uses Make/Docker; product tools call the HTTP API.

Published image: [`interchouette/kms-secp256k1-api-mcp`](https://hub.docker.com/r/interchouette/kms-secp256k1-api-mcp) (`:1.2.0`, `:latest`, `:dev`).

Cursor agents must use Cursor `CallMcpTool`. The always-apply rule lives in **itc-cursor** (product branch `kms-secp256k1-api`) as `.cursor/rules/kms-use-mcp.mdc`. Do not use Shell/`make`/curl as a substitute when MCP is ready.

## Run without compiling

```bash
# MCP HTTP sidecar (needs Docker socket + this repo mounted as workspace)
docker pull interchouette/kms-secp256k1-api-mcp:1.2.0
docker run --rm -d --name kms-secp256k1-api-mcp \
  -p 7790:7790 \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -v "$PWD":/workspace \
  -e KMS_API_ROOT=/workspace \
  interchouette/kms-secp256k1-api-mcp:1.2.0
```

Cursor config lives in product **itc-cursor** `.cursor/mcp.json` (evaluator-style launchers — no bare `url`, no host-absolute paths).

From a clone, `make mcp-http` pulls the Hub image (builds locally only if pull fails).

| Mode | How | Cursor |
| --- | --- | --- |
| **HTTP** (Docker / Hub) | `make mcp-http` | `"url": "http://127.0.0.1:7790/mcp"` |
| **HTTP** (host) | `make run-mcp-http` | same URL |
| **stdio** (host) | `make run-mcp` / cargo via `.cursor/mcp.json` | `${workspaceFolder}` spawn |

```bash
kms-secp256k1-api-mcp                         # stdio
kms-secp256k1-api-mcp --http                  # 127.0.0.1:7790
kms-secp256k1-api-mcp --http --listen 0.0.0.0:7790
```

## Make matrix (MCP itself)

| Command | Effect |
| --- | --- |
| `make mcp-build` | Host release-build MCP binary |
| `make mcp-docker-build` | Build `kms-secp256k1-api-mcp:1.2.0` (+ `:latest`) |
| `make mcp-docker-build-dev` | Build `:dev` (Hub + GHCR tags) |
| `make mcp-http` | Pull Hub image (or build) + start sidecar on **7790** |
| `make mcp-http-stop` | Stop MCP sidecar |
| `make mcp-docker-push-dev` / `mcp-docker-push-release` | Push Hub + GHCR |
| `make run-mcp` | Host **stdio** MCP |
| `make run-mcp-http` | Host HTTP on `127.0.0.1:7790` |

Compose: [`docker/docker-compose.mcp.yml`](../docker/docker-compose.mcp.yml). Image Dockerfile: [`mcp/Dockerfile`](../mcp/Dockerfile) (includes `docker`/`make`/`curl` so lifecycle tools work via the mounted repo + docker.sock).

### Published MCP images

| Registry | Image |
| --- | --- |
| Docker Hub | `interchouette/kms-secp256k1-api-mcp` |
| Personal GHCR | `ghcr.io/groussac/kms-secp256k1-api-mcp` |
| Org GHCR | `ghcr.io/interchouette-itc/kms-secp256k1-api-mcp` |

Tags: `:dev` (CI on `mcp/**` / workflow_dispatch), `:X.Y.Z` + `:latest` on `mcp/**` push to `dev` and on GitHub Release (same cadence as API/LocalStack).

### Cursor `mcp.json`

Shipped on itc-cursor product branch `kms-secp256k1-api` as `.cursor/mcp.json` (evaluator-style launchers under `.cursor/scripts/`). Reload Cursor MCP after pull. Agents must use `CallMcpTool` (rule `kms-use-mcp.mdc`).

### Tests

```bash
cd mcp
cargo test                         # unit + FakeRepo + HTTP/stdio transport
cargo test --test api_roundtrip    # host mock create/list/delete
KMS_MCP_LIVE=1 cargo test --test api_roundtrip -- --ignored --nocapture  # LocalStack stack
```

## Tool catalog

### Lifecycle (Make parity)

MCP drives **Docker/Make** from `KMS_API_ROOT`. Not exposed: `docker-push-*`, `docker-hub-description`, `version-bump-*`, `version-set`.

| Tool | Make / behavior |
| --- | --- |
| `kms_help` | `make help` + tool map |
| `kms_build` / `kms_build_release` / `kms_check` | matching targets; optional `features` |
| `kms_lint` / `kms_test` / `kms_verify` | mock suite |
| `kms_test_localstack` | `make test-localstack` (long) |
| `kms_docker_build` / `_dev` / `_no_cache` | API images |
| `kms_docker_build_localstack` / `_dev` | LocalStack images |
| `kms_docker_run` | prod compose `:4000` |
| `kms_docker_run_test` | test compose **detached** `:4001` (Make is foreground) |
| `kms_docker_stop` / `kms_docker_stop_all` | stop prod (+ tear down test) |
| `kms_docker_run_localstack` / `kms_docker_stop_localstack` | LocalStack `:4566` |
| `kms_docker_inspect` / `kms_version_show` | inspect / version |
| `kms_stack_start` / `kms_stack_stop` | test-localstack compose (API `:4001` + LocalStack) |
| `kms_api_start` / `kms_api_stop` | host `cargo run` mock API (pid under `mcp/.run/`) |
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
cargo run --manifest-path mcp/Cargo.toml --example help
cargo run --manifest-path mcp/Cargo.toml --example status
cargo run --manifest-path mcp/Cargo.toml --example roundtrip   # host mock API
```

See [`mcp/examples/README.md`](../mcp/examples/README.md).
