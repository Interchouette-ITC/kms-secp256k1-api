//! Repo root resolution and runtime paths for the MCP sidecar.

use std::env;
use std::path::{Path, PathBuf};

/// Default Streamable HTTP bind (host).
pub const DEFAULT_HTTP_LISTEN: &str = "127.0.0.1:7790";
/// Default MCP HTTP URL for docs / status.
pub const DEFAULT_MCP_URL: &str = "http://127.0.0.1:7790/mcp";
/// Default KMS HTTP API base URL (prod compose / host `APP_PORT=4000`).
pub const DEFAULT_API_URL: &str = "http://127.0.0.1:4000";
/// Test compose API port (`docker-compose.test*.yml`).
pub const DEFAULT_TEST_API_URL: &str = "http://127.0.0.1:4001";

pub const LOCALSTACK_CONTAINER: &str = "kms-localstack";
pub const LOCALSTACK_TEST_CONTAINER: &str = "kms-localstack-test";
pub const API_PROD_CONTAINER: &str = "kms-secp256k1-api";
pub const API_TEST_CONTAINER: &str = "kms-secp256k1-api-test";
pub const API_TEST_LOCALSTACK_CONTAINER: &str = "kms-secp256k1-api-test-localstack";

pub const COMPOSE_PROD: &str = "docker/docker-compose.prod.yml";
pub const COMPOSE_TEST: &str = "docker/docker-compose.test.yml";
pub const COMPOSE_LOCALSTACK: &str = "docker/docker-compose.localstack.yml";
pub const COMPOSE_TEST_LOCALSTACK: &str = "docker/docker-compose.test-localstack.yml";

/// Product compose files driven by MCP lifecycle tools (not the MCP sidecar compose).
pub const PRODUCT_COMPOSE_FILES: &[&str] = &[
    COMPOSE_PROD,
    COMPOSE_TEST,
    COMPOSE_LOCALSTACK,
    COMPOSE_TEST_LOCALSTACK,
];

/// Resolve the kms-secp256k1-api repository root.
///
/// Order: `KMS_API_ROOT` → cwd with `Makefile` → parent of cwd (when run from `mcp/`).
pub fn repo_root() -> PathBuf {
    if let Ok(env_root) = env::var("KMS_API_ROOT") {
        return PathBuf::from(env_root);
    }
    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    if cwd.join("Makefile").is_file() && cwd.join("Cargo.toml").is_file() {
        return cwd;
    }
    let parent = cwd.join("..");
    if parent.join("Makefile").is_file() && parent.join("Cargo.toml").is_file() {
        return parent.canonicalize().unwrap_or(parent);
    }
    cwd
}

/// Absolute path on the **Docker host** for `-v` bind sources.
///
/// MCP runs with the repo bind-mounted at `/workspace` and talks to the host
/// Docker socket. Paths inside the MCP container (`KMS_API_ROOT=/workspace`)
/// must **not** be passed as `-v` sources — the daemon would create host
/// `/workspace` on the root disk. Set `KMS_HOST_ROOT` to the host clone path
/// (launch scripts / compose do this). When unset (host-native MCP), falls back
/// to [`repo_root`].
///
/// Product compose has no host data binds today; this still defines the contract
/// if binds are added later.
pub fn host_repo_root() -> PathBuf {
    if let Ok(host) = env::var("KMS_HOST_ROOT") {
        let host = host.trim();
        if !host.is_empty() {
            return PathBuf::from(host);
        }
    }
    repo_root()
}

/// MCP container layout: in-container root is `/workspace` (host clone bind).
pub fn mcp_uses_workspace_mount() -> bool {
    matches!(
        env::var("KMS_API_ROOT").ok().as_deref().map(str::trim),
        Some("/workspace")
    )
}

/// True when Docker bind sources must not be used (would hit host `/`, `/workspace`, etc.).
///
/// Hard refuse for lifecycle **start** tools. In the MCP container
/// (`KMS_API_ROOT=/workspace`), `KMS_HOST_ROOT` is **required** and must be an
/// absolute host path outside `/workspace` and not `/`.
pub fn host_bind_root_is_unsafe() -> bool {
    if mcp_uses_workspace_mount() {
        match env::var("KMS_HOST_ROOT") {
            Ok(h) if !h.trim().is_empty() => {}
            _ => return true,
        }
    }

    let host = host_repo_root();
    if !host.is_absolute() {
        return true;
    }
    if host == Path::new("/") {
        return true;
    }
    if host == Path::new("/workspace") || host.starts_with("/workspace/") {
        return true;
    }
    false
}

/// Error text when refusing compose starts that would trash the host disk if binds appear.
pub fn unsafe_host_bind_message(tool: &str) -> String {
    format!(
        "REFUSING {tool}: unsafe Docker bind root (would write under host /workspace or /). \
KMS_HOST_ROOT is required when KMS_API_ROOT=/workspace and must be an absolute \
host clone path (e.g. /opt2/kms-secp256k1-api) — never /workspace and never /. \
Compose/Cursor launch scripts must pass -e KMS_HOST_ROOT=<host-path>. Product compose \
has no host data binds today; this guard blocks the latent footgun if binds are added. \
Note: docker.sock access is still host-root equivalent; this guard only blocks known \
footgun bind sources in MCP lifecycle tools."
    )
}

/// Directory for host API pid/log files (`mcp/.run/`).
pub fn run_dir() -> PathBuf {
    repo_root().join("mcp").join(".run")
}

pub fn api_pid_path() -> PathBuf {
    run_dir().join("api.pid")
}

pub fn api_log_path() -> PathBuf {
    run_dir().join("api.log")
}

/// HTTP base URL for API tools (`KMS_API_URL`, else default `:4000`).
pub fn api_base_url() -> String {
    env::var("KMS_API_URL").unwrap_or_else(|_| DEFAULT_API_URL.to_string())
}

#[cfg(test)]
#[path = "../tests/common/isolation.rs"]
pub mod test_env;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::test_env::run_isolated;

    #[test]
    fn repo_root_respects_env() {
        if run_isolated("paths::tests::repo_root_respects_env", false, |cmd| {
            cmd.env("KMS_API_ROOT", "/test/mcp-root");
        }) {
            return;
        }
        assert_eq!(repo_root(), PathBuf::from("/test/mcp-root"));
    }

    #[test]
    fn host_repo_root_prefers_kms_host_root() {
        if run_isolated(
            "paths::tests::host_repo_root_prefers_kms_host_root",
            false,
            |cmd| {
                cmd.env("KMS_API_ROOT", "/workspace")
                    .env("KMS_HOST_ROOT", "/opt2/kms-secp256k1-api");
            },
        ) {
            return;
        }
        assert_eq!(host_repo_root(), PathBuf::from("/opt2/kms-secp256k1-api"));
        assert!(!host_bind_root_is_unsafe());
    }

    #[test]
    fn host_bind_refuses_root_and_relative() {
        let test = "paths::tests::host_bind_refuses_root_and_relative";
        if env::var("KMS_MCP_ISOLATED_TEST").ok().as_deref() != Some(test) {
            for host in [
                "/",
                "relative/path",
                "/workspace",
                "/workspace/foo",
                "/opt2/kms-secp256k1-api",
            ] {
                run_isolated(test, false, |cmd| {
                    cmd.env("KMS_HOST_ROOT", host);
                });
            }
            return;
        }
        let host = env::var("KMS_HOST_ROOT").unwrap();
        assert_eq!(
            host_bind_root_is_unsafe(),
            host != "/opt2/kms-secp256k1-api"
        );
    }
}
