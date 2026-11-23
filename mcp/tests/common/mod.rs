//! Shared fake repo under `KMS_API_ROOT` for integration tests.

use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

mod isolation;
pub use isolation::run_isolated;

pub struct FakeRepo {
    pub root: PathBuf,
}

impl FakeRepo {
    pub fn new() -> Self {
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
        fs::write(
            root.join("docker/docker-compose.prod.yml"),
            "services: {}\n",
        )
        .unwrap();
        fs::write(
            root.join("docker/docker-compose.test.yml"),
            "services: {}\n",
        )
        .unwrap();
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

        Self { root }
    }
}

impl Drop for FakeRepo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
