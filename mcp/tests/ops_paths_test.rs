mod common;

use common::{FakeRepo, run_isolated};
use kms_secp256k1_api_mcp::ops;
use kms_secp256k1_api_mcp::paths::{
    api_base_url, repo_root, COMPOSE_LOCALSTACK, COMPOSE_PROD, COMPOSE_TEST,
    COMPOSE_TEST_LOCALSTACK, DEFAULT_API_URL, DEFAULT_MCP_URL, DEFAULT_TEST_API_URL,
};

#[test]
fn defaults_are_stable() {
    assert_eq!(DEFAULT_API_URL, "http://127.0.0.1:4000");
    assert_eq!(DEFAULT_TEST_API_URL, "http://127.0.0.1:4001");
    assert!(DEFAULT_MCP_URL.contains("7790"));
}

#[test]
fn help_with_fake_repo() {
    let fx = FakeRepo::new();
    if run_isolated("help_with_fake_repo", false, |cmd| {
        cmd.env("KMS_API_ROOT", &fx.root)
            .env("KMS_HOST_ROOT", &fx.root);
    }) {
        return;
    }
    let text = ops::help();
    assert!(text.contains("kms_build"), "{text}");
    assert!(text.contains("kms_stack_start"), "{text}");
    assert!(text.contains("kms-secp256k1-api targets"), "{text}");
}

#[test]
fn version_show_with_fake_repo() {
    let fx = FakeRepo::new();
    if run_isolated("version_show_with_fake_repo", false, |cmd| {
        cmd.env("KMS_API_ROOT", &fx.root)
            .env("KMS_HOST_ROOT", &fx.root);
    }) {
        return;
    }
    let text = ops::version_show();
    assert!(text.contains("0.0.0") || text.contains("ok"), "{text}");
}

#[test]
fn status_with_fake_repo() {
    let fx = FakeRepo::new();
    if run_isolated("status_with_fake_repo", false, |cmd| {
        cmd.env("KMS_API_ROOT", &fx.root)
            .env("KMS_HOST_ROOT", &fx.root);
    }) {
        return;
    }
    let text = ops::status();
    assert!(text.contains("repo_root="), "{text}");
    assert!(text.contains("containers"), "{text}");
}

#[test]
fn repo_root_from_env() {
    let fx = FakeRepo::new();
    if run_isolated("repo_root_from_env", false, |cmd| {
        cmd.env("KMS_API_ROOT", &fx.root);
    }) {
        return;
    }
    assert_eq!(
        repo_root(),
        std::path::PathBuf::from(std::env::var_os("KMS_API_ROOT").unwrap())
    );
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
    if run_isolated("api_base_url_default", false, |_| {}) {
        return;
    }
    assert_eq!(api_base_url(), DEFAULT_API_URL);
}

#[test]
fn docker_run_refuses_workspace_host_binds() {
    if run_isolated("docker_run_refuses_workspace_host_binds", false, |cmd| {
        cmd.env("KMS_API_ROOT", "/workspace");
    }) {
        return;
    }
    for (name, out) in [
        ("kms_docker_run", ops::docker_run()),
        ("kms_docker_run_test", ops::docker_run_test()),
        ("kms_docker_run_localstack", ops::docker_run_localstack()),
        ("kms_stack_start", ops::stack_start()),
    ] {
        assert!(out.contains("REFUSING"), "{name}: {out}");
        assert!(out.contains("KMS_HOST_ROOT"), "{name}: {out}");
    }
}

#[test]
fn safe_host_root_does_not_refuse() {
    use kms_secp256k1_api_mcp::paths::host_bind_root_is_unsafe;

    let fx = FakeRepo::new();
    if run_isolated("safe_host_root_does_not_refuse", false, |cmd| {
        cmd.env("KMS_API_ROOT", "/workspace")
            .env("KMS_HOST_ROOT", &fx.root);
    }) {
        return;
    }
    assert!(!host_bind_root_is_unsafe());
}
