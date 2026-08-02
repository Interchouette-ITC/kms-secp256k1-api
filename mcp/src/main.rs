//! `kms-secp256k1-api-mcp` — MCP server (stdio by default, optional Streamable HTTP).

use anyhow::Result;
use clap::Parser;
use kms_secp256k1_api_mcp::server::{run, run_http, DEFAULT_HTTP_LISTEN};

#[derive(Debug, Parser)]
#[command(
    name = "kms-secp256k1-api-mcp",
    about = "kms-secp256k1-api MCP server (stdio or Streamable HTTP)",
    version
)]
struct Cli {
    /// Serve Streamable HTTP instead of stdio (also: `KMS_MCP_HTTP=1`).
    #[arg(long, env = "KMS_MCP_HTTP")]
    http: bool,

    /// HTTP bind address when `--http` is set (also: `KMS_MCP_ADDR`).
    #[arg(long, env = "KMS_MCP_ADDR", default_value = DEFAULT_HTTP_LISTEN)]
    listen: String,
}

fn init_logging() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .init();
}

#[tokio::main]
async fn main() -> Result<()> {
    init_logging();
    let cli = Cli::parse();

    if cli.http {
        tracing::info!(addr = %cli.listen, "kms-secp256k1-api-mcp starting (HTTP)");
        run_http(&cli.listen)
            .await
            .map_err(|err| anyhow::anyhow!("{err}"))?;
    } else {
        tracing::info!("kms-secp256k1-api-mcp starting (stdio)");
        run().await.map_err(|err| anyhow::anyhow!("{err}"))?;
    }
    Ok(())
}
