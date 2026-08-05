# MCP examples (developers / CI)

These binaries call the **same Rust helpers** the MCP server uses (`ops`, `client`). They are for **local smoke and CI**.

```bash
export KMS_API_ROOT=/path/to/kms-secp256k1-api
export KMS_HOST_ROOT=/path/to/kms-secp256k1-api

cargo run --manifest-path mcp/Cargo.toml --example help
cargo run --manifest-path mcp/Cargo.toml --example status
cargo run --manifest-path mcp/Cargo.toml --example api_hello
cargo run --manifest-path mcp/Cargo.toml --example localstack_start_stop   # needs Docker
cargo run --manifest-path mcp/Cargo.toml --example roundtrip               # host mock API
```

`localstack_start_stop` needs Docker + LocalStack image build capability.
`roundtrip` compiles/runs the API crate via `cargo run` (mock `TESTING_MODE`).
