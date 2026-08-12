//! KMS secp256k1 API MCP library (rmcp tools + Make/Docker helpers + HTTP client).

pub mod client;
pub mod ops;
pub mod paths;
pub mod server;
pub mod tool_args;

pub use server::{DEFAULT_HTTP_LISTEN, run, run_http};
