//! Shared fake repo under `KMS_API_ROOT` for integration tests.

use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

pub struct FakeRepo {
    pub root: PathBuf,
    _guard: std::sync::MutexGuard<'static, ()>,
}

impl FakeRepo {
    pub fn new() -> Self {
        let guard = env_lock().lock().unwrap_or_else(|e| e.into_inner());
        let mut root = env::temp_dir();
        root.push(format!(
            "kms-mcp-fixture-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("docker")).unwrap();
        fs::write(
            root.join("Makefile"),
            "help:\n\t@echo kms-secp256k1-api targets\n\
version-show:\n\t@echo Current version: 0.0.0\n\
docker-inspect:\n\t@echo Image not found\n\
docker-stop:\n\t@echo stopped\n",
        )
        .unwrap();
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname=\"kms-secp256k1-api\"\nversion=\"0.0.0\"\nedition=\"2021\"\n",
        )
        .unwrap();
        fs::write(root.join("docker/docker-compose.prod.yml"), "services: {}\n").unwrap();
        fs::write(root.join("docker/docker-compose.test.yml"), "services: {}\n").unwrap();
        fs::write(
            root.join("docker/docker-compose.localstack.yml"),
            "services: {}\n",
        )
        .unwrap();
        fs::write(
            root.join("docker/docker-compose.test-localstack.yml"),
            "services: {}\n",
        )
        .unwrap();

        env::set_var("KMS_API_ROOT", &root);
        env::set_var("KMS_HOST_ROOT", &root);
        Self {
            root,
            _guard: guard,
        }
    }
}

impl Drop for FakeRepo {
    fn drop(&mut self) {
        env::remove_var("KMS_API_ROOT");
        env::remove_var("KMS_HOST_ROOT");
        let _ = fs::remove_dir_all(&self.root);
    }
}
