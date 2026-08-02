//! MCP server (`mcpkit`) for `kms-secp256k1-api-mcp` (stdio or Streamable HTTP).

#![allow(clippy::unused_async)]

use mcpkit::prelude::*;
use mcpkit::transport::stdio::StdioTransport;
use mcpkit_axum::McpRouter;

use crate::{client, ops};

/// MCP server handle exposing KMS Make/Docker + HTTP API tools.
pub struct KmsMcp;

// Keep in sync with Cargo.toml `version`.
#[mcp_server(name = "kms-secp256k1-api", version = "1.1.0")]
impl KmsMcp {
    #[tool(description = "make help + MCP Make↔tool parity map")]
    async fn kms_help(&self) -> ToolOutput {
        ToolOutput::text(ops::help())
    }

    #[tool(description = "make build [FEATURES=…] — cargo build API")]
    async fn kms_build(&self, features: Option<String>) -> ToolOutput {
        ToolOutput::text(ops::build(features.as_deref()))
    }

    #[tool(description = "make build-release [FEATURES=…]")]
    async fn kms_build_release(&self, features: Option<String>) -> ToolOutput {
        ToolOutput::text(ops::build_release(features.as_deref()))
    }

    #[tool(description = "make check [FEATURES=…]")]
    async fn kms_check(&self, features: Option<String>) -> ToolOutput {
        ToolOutput::text(ops::check(features.as_deref()))
    }

    #[tool(description = "make lint [FEATURES=…] — fmt check + clippy")]
    async fn kms_lint(&self, features: Option<String>) -> ToolOutput {
        ToolOutput::text(ops::lint(features.as_deref()))
    }

    #[tool(description = "make test [FEATURES=…] — mock KMS suite")]
    async fn kms_test(&self, features: Option<String>) -> ToolOutput {
        ToolOutput::text(ops::test_mock(features.as_deref()))
    }

    #[tool(description = "make verify [FEATURES=…] — lint + mock tests")]
    async fn kms_verify(&self, features: Option<String>) -> ToolOutput {
        ToolOutput::text(ops::verify(features.as_deref()))
    }

    #[tool(
        description = "make test-localstack [FEATURES=…] — integration suite vs LocalStack (long)"
    )]
    async fn kms_test_localstack(&self, features: Option<String>) -> ToolOutput {
        ToolOutput::text(ops::test_localstack(features.as_deref()))
    }

    #[tool(description = "make docker-build — API image :latest + version tags")]
    async fn kms_docker_build(&self) -> ToolOutput {
        ToolOutput::text(ops::docker_build())
    }

    #[tool(description = "make docker-build-dev — API image :dev tags")]
    async fn kms_docker_build_dev(&self) -> ToolOutput {
        ToolOutput::text(ops::docker_build_dev())
    }

    #[tool(description = "make docker-build-no-cache")]
    async fn kms_docker_build_no_cache(&self) -> ToolOutput {
        ToolOutput::text(ops::docker_build_no_cache())
    }

    #[tool(description = "make docker-build-localstack")]
    async fn kms_docker_build_localstack(&self) -> ToolOutput {
        ToolOutput::text(ops::docker_build_localstack())
    }

    #[tool(description = "make docker-build-localstack-dev")]
    async fn kms_docker_build_localstack_dev(&self) -> ToolOutput {
        ToolOutput::text(ops::docker_build_localstack_dev())
    }

    #[tool(description = "make docker-run — prod compose up -d (port 4000)")]
    async fn kms_docker_run(&self) -> ToolOutput {
        ToolOutput::text(ops::docker_run())
    }

    #[tool(
        description = "docker-run-test parity: test compose up -d (port 4001, mock). Make is foreground; MCP detaches."
    )]
    async fn kms_docker_run_test(&self) -> ToolOutput {
        ToolOutput::text(ops::docker_run_test())
    }

    #[tool(description = "make docker-stop — stop prod compose")]
    async fn kms_docker_stop(&self) -> ToolOutput {
        ToolOutput::text(ops::docker_stop())
    }

    #[tool(description = "Stop prod + tear down test compose")]
    async fn kms_docker_stop_all(&self) -> ToolOutput {
        ToolOutput::text(ops::docker_stop_all_api())
    }

    #[tool(description = "make docker-run-localstack — LocalStack KMS on :4566")]
    async fn kms_docker_run_localstack(&self) -> ToolOutput {
        ToolOutput::text(ops::docker_run_localstack())
    }

    #[tool(description = "make docker-stop-localstack")]
    async fn kms_docker_stop_localstack(&self) -> ToolOutput {
        ToolOutput::text(ops::docker_stop_localstack())
    }

    #[tool(description = "make docker-inspect — image tags/size")]
    async fn kms_docker_inspect(&self) -> ToolOutput {
        ToolOutput::text(ops::docker_inspect())
    }

    #[tool(description = "make version-show")]
    async fn kms_version_show(&self) -> ToolOutput {
        ToolOutput::text(ops::version_show())
    }

    #[tool(
        description = "Start LocalStack + API (test-localstack compose, API :4001). Builds images if needed."
    )]
    async fn kms_stack_start(&self) -> ToolOutput {
        ToolOutput::text(ops::stack_start())
    }

    #[tool(description = "Stop LocalStack + API test-localstack compose")]
    async fn kms_stack_stop(&self) -> ToolOutput {
        ToolOutput::text(ops::stack_stop())
    }

    #[tool(
        description = "Build (if needed) and start host release API in background (TESTING_MODE=true mock). Optional features (default casper), port."
    )]
    async fn kms_api_start(
        &self,
        features: Option<String>,
        port: Option<u32>,
    ) -> ToolOutput {
        let port = port.and_then(|p| u16::try_from(p).ok());
        ToolOutput::text(ops::api_start(features.as_deref(), port))
    }

    #[tool(description = "Stop host cargo API started by kms_api_start")]
    async fn kms_api_stop(&self) -> ToolOutput {
        ToolOutput::text(ops::api_stop())
    }

    #[tool(description = "Container status + GET / probes on :4000 and :4001")]
    async fn kms_status(&self) -> ToolOutput {
        ToolOutput::text(ops::status())
    }

    // --- HTTP API tools ---

    #[tool(description = "GET / — hello (+ optional message query)")]
    async fn kms_hello(&self, message: Option<String>) -> ToolOutput {
        ToolOutput::text(client::hello(message.as_deref()).await)
    }

    #[tool(description = "POST /createKey — create secp256k1 key via KMS/mock")]
    async fn kms_create_key(&self) -> ToolOutput {
        ToolOutput::text(client::create_key().await)
    }

    #[tool(
        description = "POST /signTransactionHash?keys=… with text/plain hex hash body"
    )]
    async fn kms_sign_transaction_hash(&self, keys: String, hash_hex: String) -> ToolOutput {
        ToolOutput::text(client::sign_transaction_hash(&keys, &hash_hex).await)
    }

    #[tool(description = "POST /signTransaction?keys=… with JSON transaction body")]
    async fn kms_sign_transaction(&self, keys: String, tx_json: String) -> ToolOutput {
        ToolOutput::text(client::sign_transaction(&keys, &tx_json).await)
    }

    #[tool(
        description = "GET /verifySignature — key, transaction_hash, signature; optional via_kms"
    )]
    async fn kms_verify_signature(
        &self,
        key: String,
        transaction_hash: String,
        signature: String,
        via_kms: Option<bool>,
    ) -> ToolOutput {
        ToolOutput::text(
            client::verify_signature(&key, &transaction_hash, &signature, via_kms).await,
        )
    }

    #[tool(description = "DELETE /deleteKey?key=… (requires DELETE_MODE=true)")]
    async fn kms_delete_key(&self, key: String) -> ToolOutput {
        ToolOutput::text(client::delete_key(&key).await)
    }

    #[tool(description = "GET /listKeys (requires LIST_MODE=true)")]
    async fn kms_list_keys(&self) -> ToolOutput {
        ToolOutput::text(client::list_keys().await)
    }

    #[tool(description = "GET /docs/openapi.json — summarize title/version/paths")]
    async fn kms_openapi(&self) -> ToolOutput {
        ToolOutput::text(client::openapi().await)
    }
}

/// Serves MCP over stdio until the client disconnects.
pub async fn run() -> Result<(), McpError> {
    let transport = StdioTransport::new();
    let server = ServerBuilder::new(KmsMcp)
        .with_tools(KmsMcp)
        .build();
    server.serve(transport).await
}

/// Default HTTP bind address for Streamable MCP.
pub const DEFAULT_HTTP_LISTEN: &str = crate::paths::DEFAULT_HTTP_LISTEN;

/// Serves MCP over Streamable HTTP until the process is stopped.
pub async fn run_http(addr: &str) -> std::io::Result<()> {
    McpRouter::new(KmsMcp).serve(addr).await
}

impl ResourceHandler for KmsMcp {
    async fn list_resources(&self, _ctx: &Context<'_>) -> Result<Vec<Resource>, McpError> {
        Ok(Vec::new())
    }

    async fn read_resource(
        &self,
        uri: &str,
        _ctx: &Context<'_>,
    ) -> Result<Vec<ResourceContents>, McpError> {
        Err(McpError::invalid_params(
            "resources/read",
            format!("unknown resource: {uri}"),
        ))
    }
}

impl PromptHandler for KmsMcp {
    async fn list_prompts(&self, _ctx: &Context<'_>) -> Result<Vec<Prompt>, McpError> {
        Ok(Vec::new())
    }

    async fn get_prompt(
        &self,
        name: &str,
        _args: Option<serde_json::Map<String, serde_json::Value>>,
        _ctx: &Context<'_>,
    ) -> Result<GetPromptResult, McpError> {
        Err(McpError::invalid_params(
            "prompts/get",
            format!("unknown prompt: {name}"),
        ))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn mcp_server_version_matches_crate() {
        assert_eq!(
            env!("CARGO_PKG_VERSION"),
            "1.1.0",
            "bump #[mcp_server(version = …)] when changing Cargo.toml version"
        );
    }
}
