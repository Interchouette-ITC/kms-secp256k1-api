# MCP server for kms-secp256k1-api

Rust **mcpkit** server (`kms-secp256k1-api-mcp` **v1.2.0**) to drive Make/Docker lifecycle and call the HTTP API from Cursor.

This is a **separate Cargo package** under `mcp/` — it does **not** link the API library. Lifecycle shells out to `make`/`docker`; API tools use `reqwest`.

## Transports

| Mode | Command | Use |
| --- | --- | --- |
| **stdio (Hub)** | `docker run -i … interchouette/kms-secp256k1-api-mcp:1.2.0` | No local Rust; see [mcp.json.example](mcp.json.example) |
| **stdio (host)** | `make run-mcp` / `kms-secp256k1-api-mcp` | Local Cursor spawn |
| **HTTP (host)** | `make run-mcp-http` | Streamable HTTP on **10889** |
| **HTTP (Docker)** | `make mcp-http` | Pull Hub (or build) + compose on **10889** → `http://127.0.0.1:10889/mcp` |

```bash
docker pull interchouette/kms-secp256k1-api-mcp:1.2.0
make mcp-http           # pull-first sidecar
make mcp-http-stop
make mcp-docker-build   # local image if needed
```

See [docs/mcp.md](../docs/mcp.md) and [mcp.json.example](mcp.json.example).

## Env

| Var | Default | Role |
| --- | --- | --- |
| `KMS_API_ROOT` | parent of `mcp/` / cwd with Makefile | Repo root for Make/Docker |
| `KMS_API_URL` | `http://127.0.0.1:4000` | HTTP tools base URL (use `:4001` for test compose) |
| `KMS_MCP_HTTP` | unset | Force HTTP transport |
| `KMS_MCP_ADDR` | `127.0.0.1:10889` | HTTP listen address |

## Tests & examples

```bash
cd mcp
cargo test --all-targets
cargo build --examples
```

Examples call the same helpers as the MCP tools (`ops` / `client`). See [examples/README.md](examples/README.md). Cursor agents must still use `CallMcpTool` (see `.cursor/rules/kms-use-mcp.mdc`).
