//! MCP server (`rmcp`) for `kms-secp256k1-api-mcp` (stdio or Streamable HTTP).

#![allow(clippy::unused_async)]

use std::sync::Arc;

use rmcp::{
    ErrorData as McpError, ServerHandler, ServiceExt,
    handler::server::wrapper::Parameters,
    model::{CallToolResult, ContentBlock, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
    transport::stdio,
};

use crate::tool_args::*;
use crate::{client, ops};

/// MCP server handle exposing KMS Make/Docker + HTTP API tools.
#[derive(Clone, Default)]
pub struct KmsMcp;

/// Default HTTP bind address for Streamable MCP.
pub const DEFAULT_HTTP_LISTEN: &str = crate::paths::DEFAULT_HTTP_LISTEN;

fn text_ok(text: impl Into<String>) -> CallToolResult {
    CallToolResult::success(vec![ContentBlock::text(text.into())])
}

#[tool_router]
impl KmsMcp {
    #[tool(description = "make help + MCP Make↔tool parity map")]
    async fn kms_help(&self) -> Result<CallToolResult, McpError> {
        Ok(text_ok(ops::help()))
    }

    #[tool(description = "make build [FEATURES=…] - cargo build API")]
    async fn kms_build(
        &self,
        Parameters(FeaturesArgs { features }): Parameters<FeaturesArgs>,
    ) -> Result<CallToolResult, McpError> {
        Ok(text_ok(ops::build(features.as_deref())))
    }

    #[tool(description = "make build-release [FEATURES=…]")]
    async fn kms_build_release(
        &self,
        Parameters(FeaturesArgs { features }): Parameters<FeaturesArgs>,
    ) -> Result<CallToolResult, McpError> {
        Ok(text_ok(ops::build_release(features.as_deref())))
    }

    #[tool(description = "make check [FEATURES=…]")]
    async fn kms_check(
        &self,
        Parameters(FeaturesArgs { features }): Parameters<FeaturesArgs>,
    ) -> Result<CallToolResult, McpError> {
        Ok(text_ok(ops::check(features.as_deref())))
    }

    #[tool(description = "make lint [FEATURES=…] - fmt check + clippy")]
    async fn kms_lint(
        &self,
        Parameters(FeaturesArgs { features }): Parameters<FeaturesArgs>,
    ) -> Result<CallToolResult, McpError> {
        Ok(text_ok(ops::lint(features.as_deref())))
    }

    #[tool(description = "make test [FEATURES=…] - mock KMS suite")]
    async fn kms_test(
        &self,
        Parameters(FeaturesArgs { features }): Parameters<FeaturesArgs>,
    ) -> Result<CallToolResult, McpError> {
        Ok(text_ok(ops::test_mock(features.as_deref())))
    }

    #[tool(description = "make verify [FEATURES=…] - lint + mock tests")]
    async fn kms_verify(
        &self,
        Parameters(FeaturesArgs { features }): Parameters<FeaturesArgs>,
    ) -> Result<CallToolResult, McpError> {
        Ok(text_ok(ops::verify(features.as_deref())))
    }

    #[tool(
        description = "make test-localstack [FEATURES=…] - integration suite vs LocalStack (long)"
    )]
    async fn kms_test_localstack(
        &self,
        Parameters(FeaturesArgs { features }): Parameters<FeaturesArgs>,
    ) -> Result<CallToolResult, McpError> {
        Ok(text_ok(ops::test_localstack(features.as_deref())))
    }

    #[tool(description = "make docker-build - API image :latest + version tags")]
    async fn kms_docker_build(&self) -> Result<CallToolResult, McpError> {
        Ok(text_ok(ops::docker_build()))
    }

    #[tool(description = "make docker-build-dev - API image :dev tags")]
    async fn kms_docker_build_dev(&self) -> Result<CallToolResult, McpError> {
        Ok(text_ok(ops::docker_build_dev()))
    }

    #[tool(description = "make docker-build-no-cache")]
    async fn kms_docker_build_no_cache(&self) -> Result<CallToolResult, McpError> {
        Ok(text_ok(ops::docker_build_no_cache()))
    }

    #[tool(description = "make docker-build-localstack")]
    async fn kms_docker_build_localstack(&self) -> Result<CallToolResult, McpError> {
        Ok(text_ok(ops::docker_build_localstack()))
    }

    #[tool(description = "make docker-build-localstack-dev")]
    async fn kms_docker_build_localstack_dev(&self) -> Result<CallToolResult, McpError> {
        Ok(text_ok(ops::docker_build_localstack_dev()))
    }

    #[tool(description = "make docker-run - prod compose up -d (port 4000)")]
    async fn kms_docker_run(&self) -> Result<CallToolResult, McpError> {
        Ok(text_ok(ops::docker_run()))
    }

    #[tool(
        description = "docker-run-test parity: test compose up -d (port 4001, mock). Make is foreground; MCP detaches."
    )]
    async fn kms_docker_run_test(&self) -> Result<CallToolResult, McpError> {
        Ok(text_ok(ops::docker_run_test()))
    }

    #[tool(description = "make docker-stop - stop prod compose")]
    async fn kms_docker_stop(&self) -> Result<CallToolResult, McpError> {
        Ok(text_ok(ops::docker_stop()))
    }

    #[tool(description = "Stop prod + tear down test compose")]
    async fn kms_docker_stop_all(&self) -> Result<CallToolResult, McpError> {
        Ok(text_ok(ops::docker_stop_all_api()))
    }

    #[tool(description = "make docker-run-localstack - LocalStack KMS on :4566")]
    async fn kms_docker_run_localstack(&self) -> Result<CallToolResult, McpError> {
        Ok(text_ok(ops::docker_run_localstack()))
    }

    #[tool(description = "make docker-stop-localstack")]
    async fn kms_docker_stop_localstack(&self) -> Result<CallToolResult, McpError> {
        Ok(text_ok(ops::docker_stop_localstack()))
    }

    #[tool(description = "make docker-inspect - image tags/size")]
    async fn kms_docker_inspect(&self) -> Result<CallToolResult, McpError> {
        Ok(text_ok(ops::docker_inspect()))
    }

    #[tool(description = "make version-show")]
    async fn kms_version_show(&self) -> Result<CallToolResult, McpError> {
        Ok(text_ok(ops::version_show()))
    }

    #[tool(
        description = "Start LocalStack + API (test-localstack compose, API :4001). Builds images if needed."
    )]
    async fn kms_stack_start(&self) -> Result<CallToolResult, McpError> {
        Ok(text_ok(ops::stack_start()))
    }

    #[tool(description = "Stop LocalStack + API test-localstack compose")]
    async fn kms_stack_stop(&self) -> Result<CallToolResult, McpError> {
        Ok(text_ok(ops::stack_stop()))
    }

    #[tool(
        description = "Build (if needed) and start host release API in background (TESTING_MODE=true mock). Optional features (default casper), port."
    )]
    async fn kms_api_start(
        &self,
        Parameters(ApiStartArgs { features, port }): Parameters<ApiStartArgs>,
    ) -> Result<CallToolResult, McpError> {
        let port = port.and_then(|p| u16::try_from(p).ok());
        Ok(text_ok(ops::api_start(features.as_deref(), port)))
    }

    #[tool(description = "Stop host cargo API started by kms_api_start")]
    async fn kms_api_stop(&self) -> Result<CallToolResult, McpError> {
        Ok(text_ok(ops::api_stop()))
    }

    #[tool(description = "Container status + GET / probes on :4000 and :4001")]
    async fn kms_status(&self) -> Result<CallToolResult, McpError> {
        Ok(text_ok(ops::status()))
    }

    // --- HTTP API tools ---

    #[tool(description = "GET / - hello (+ optional message query)")]
    async fn kms_hello(
        &self,
        Parameters(HelloArgs { message }): Parameters<HelloArgs>,
    ) -> Result<CallToolResult, McpError> {
        Ok(text_ok(client::hello(message.as_deref()).await))
    }

    #[tool(description = "POST /createKey - create secp256k1 key via KMS/mock")]
    async fn kms_create_key(&self) -> Result<CallToolResult, McpError> {
        Ok(text_ok(client::create_key().await))
    }

    #[tool(description = "POST /signTransactionHash?keys=… with text/plain hex hash body")]
    async fn kms_sign_transaction_hash(
        &self,
        Parameters(SignTransactionHashArgs { keys, hash_hex }): Parameters<SignTransactionHashArgs>,
    ) -> Result<CallToolResult, McpError> {
        Ok(text_ok(
            client::sign_transaction_hash(&keys, &hash_hex).await,
        ))
    }

    #[tool(description = "POST /signTransaction?keys=… with JSON transaction body")]
    async fn kms_sign_transaction(
        &self,
        Parameters(SignTransactionArgs { keys, tx_json }): Parameters<SignTransactionArgs>,
    ) -> Result<CallToolResult, McpError> {
        Ok(text_ok(client::sign_transaction(&keys, &tx_json).await))
    }

    #[tool(
        description = "GET /verifySignature - key, transaction_hash, signature; optional via_kms"
    )]
    async fn kms_verify_signature(
        &self,
        Parameters(VerifySignatureArgs {
            key,
            transaction_hash,
            signature,
            via_kms,
        }): Parameters<VerifySignatureArgs>,
    ) -> Result<CallToolResult, McpError> {
        Ok(text_ok(
            client::verify_signature(&key, &transaction_hash, &signature, via_kms).await,
        ))
    }

    #[tool(description = "DELETE /deleteKey?key=… (requires DELETE_MODE=true)")]
    async fn kms_delete_key(
        &self,
        Parameters(DeleteKeyArgs { key }): Parameters<DeleteKeyArgs>,
    ) -> Result<CallToolResult, McpError> {
        Ok(text_ok(client::delete_key(&key).await))
    }

    #[tool(description = "GET /listKeys (requires LIST_MODE=true)")]
    async fn kms_list_keys(&self) -> Result<CallToolResult, McpError> {
        Ok(text_ok(client::list_keys().await))
    }

    #[tool(description = "GET /docs/openapi.json - summarize title/version/paths")]
    async fn kms_openapi(&self) -> Result<CallToolResult, McpError> {
        Ok(text_ok(client::openapi().await))
    }
}

/// Serves MCP over stdio until the client disconnects.
pub async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let server = KmsMcp;
    let service = server.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}

/// Serves MCP over Streamable HTTP until the process is stopped.
pub async fn run_http(addr: &str) -> std::io::Result<()> {
    let config =
        rmcp::transport::streamable_http_server::tower::StreamableHttpServerConfig::default();
    let service = rmcp::transport::streamable_http_server::tower::StreamableHttpService::new(
        || Ok(KmsMcp),
        Arc::new(
            rmcp::transport::streamable_http_server::session::local::LocalSessionManager::default(),
        ),
        config,
    );
    let method_router = axum::routing::any_service(service);
    let app = axum::Router::new()
        .route("/mcp", method_router.clone())
        .route("/mcp/", method_router);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(%addr, "kms-secp256k1-api-mcp HTTP listening");
    axum::serve(listener, app).await?;
    Ok(())
}

#[tool_handler]
impl ServerHandler for KmsMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(rmcp::model::Implementation::new(
                "kms-secp256k1-api",
                env!("CARGO_PKG_VERSION"),
            ))
            .with_instructions(
                "MCP tools for kms-secp256k1-api: Make/Docker lifecycle, LocalStack stack, and HTTP API (create/sign/verify/list/delete). Set KMS_API_ROOT / KMS_HOST_ROOT for lifecycle tools; KMS_API_URL for HTTP tools.",
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mcp_server_version_matches_crate() {
        let info = KmsMcp.get_info();
        assert_eq!(info.server_info.version, env!("CARGO_PKG_VERSION"));
        assert_eq!(info.server_info.name.as_str(), "kms-secp256k1-api");
    }
}
