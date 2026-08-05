//! Repo root resolution and runtime paths for the MCP sidecar.

use std::env;
use std::path::PathBuf;

/// Default Streamable HTTP bind (host). Avoids nctl `:8788` / tvscreener `:8787`.
pub const DEFAULT_HTTP_LISTEN: &str = "127.0.0.1:9789";
/// Default MCP HTTP URL for docs / status.
pub const DEFAULT_MCP_URL: &str = "http://127.0.0.1:9789/mcp";
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
pub mod test_env {
    use std::sync::Mutex;
    /// Serialize tests that mutate `KMS_API_ROOT` / `KMS_API_URL`.
    pub static ENV_LOCK: Mutex<()> = Mutex::new(());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::test_env::ENV_LOCK;
    use std::fs;

    #[test]
    fn repo_root_respects_env() {
        let _g = ENV_LOCK.lock().unwrap();
        let mut dir = env::temp_dir();
        dir.push(format!("kms-mcp-root-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("Makefile"), "help:\n\t@echo ok\n").unwrap();
        fs::write(
            dir.join("Cargo.toml"),
            "[package]\nname=\"x\"\nversion=\"0.0.0\"\nedition=\"2021\"\n",
        )
        .unwrap();
        env::set_var("KMS_API_ROOT", &dir);
        assert_eq!(repo_root(), dir);
        env::remove_var("KMS_API_ROOT");
        let _ = fs::remove_dir_all(&dir);
    }
}
