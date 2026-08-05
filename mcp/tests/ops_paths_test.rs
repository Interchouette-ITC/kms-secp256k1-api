mod common;

use common::FakeRepo;
use kms_secp256k1_api_mcp::ops;
use kms_secp256k1_api_mcp::paths::{
    api_base_url, repo_root, COMPOSE_LOCALSTACK, COMPOSE_PROD, COMPOSE_TEST,
    COMPOSE_TEST_LOCALSTACK, DEFAULT_API_URL, DEFAULT_MCP_URL, DEFAULT_TEST_API_URL,
};

#[test]
fn defaults_are_stable() {
    assert_eq!(DEFAULT_API_URL, "http://127.0.0.1:4000");
    assert_eq!(DEFAULT_TEST_API_URL, "http://127.0.0.1:4001");
    assert!(DEFAULT_MCP_URL.contains("9789"));
}

#[test]
fn help_with_fake_repo() {
    let _fx = FakeRepo::new();
    let text = ops::help();
    assert!(text.contains("kms_build"), "{text}");
    assert!(text.contains("kms_stack_start"), "{text}");
    assert!(text.contains("kms-secp256k1-api targets"), "{text}");
}

#[test]
fn version_show_with_fake_repo() {
    let _fx = FakeRepo::new();
    let text = ops::version_show();
    assert!(text.contains("0.0.0") || text.contains("ok"), "{text}");
}

#[test]
fn status_with_fake_repo() {
    let _fx = FakeRepo::new();
    let text = ops::status();
    assert!(text.contains("repo_root="), "{text}");
    assert!(text.contains("containers"), "{text}");
}

#[test]
fn repo_root_from_env() {
    let fx = FakeRepo::new();
    assert_eq!(repo_root(), fx.root);
}

#[test]
fn compose_path_constants() {
    assert_eq!(COMPOSE_PROD, "docker/docker-compose.prod.yml");
    assert_eq!(COMPOSE_TEST, "docker/docker-compose.test.yml");
    assert_eq!(COMPOSE_LOCALSTACK, "docker/docker-compose.localstack.yml");
    assert_eq!(
        COMPOSE_TEST_LOCALSTACK,
        "docker/docker-compose.test-localstack.yml"
    );
}

#[test]
fn api_base_url_default() {
    let _fx = FakeRepo::new();
    std::env::remove_var("KMS_API_URL");
    assert_eq!(api_base_url(), DEFAULT_API_URL);
}
