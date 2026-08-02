# MCP for agents

Rust binary `kms-secp256k1-api-mcp` **v1.1.0** (mcpkit), dual transport like casper-nctl-2-docker / tvscreener.

**Separate package:** lives in [`mcp/`](../mcp/) — **no dependency** on the `kms-secp256k1-api` library crate. Lifecycle uses Make/Docker; product tools call the HTTP API.

Cursor agents must use Cursor `CallMcpTool`. The always-apply rule lives in **itc-cursor** (product branch `kms-secp256k1-api`) as `.cursor/rules/kms-use-mcp.mdc`. Do not use Shell/`make`/curl as a substitute when MCP is ready.

| Mode | How | Cursor |
| --- | --- | --- |
| **HTTP** (Docker) | `make mcp-http` | `"url": "http://127.0.0.1:8789/mcp"` |
| **HTTP** (host) | `make run-mcp-http` | same URL |
| **stdio** (host) | `make run-mcp` / `kms-secp256k1-api-mcp` | command spawn (see example) |

```bash
kms-secp256k1-api-mcp                         # stdio
kms-secp256k1-api-mcp --http                  # 127.0.0.1:8789
kms-secp256k1-api-mcp --http --listen 0.0.0.0:8789
```

Example Cursor config: [`mcp/mcp.json.example`](../mcp/mcp.json.example).

## Make matrix (MCP itself)

| Command | Effect |
| --- | --- |
| `make mcp-build` | Host release-build MCP binary |
| `make mcp-docker-build` | Build `kms-secp256k1-api-mcp:1.1.0` (+ `:latest`) |
| `make mcp-http` | Build image + start sidecar on **8789** |
| `make mcp-http-stop` | Stop MCP sidecar |
| `make run-mcp` | Host **stdio** MCP |
| `make run-mcp-http` | Host HTTP on `127.0.0.1:8789` |

Compose: [`docker/docker-compose.mcp.yml`](../docker/docker-compose.mcp.yml). Image Dockerfile: [`mcp/Dockerfile`](../mcp/Dockerfile) (includes `docker`/`make`/`curl` so lifecycle tools work via the mounted repo + docker.sock).

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
