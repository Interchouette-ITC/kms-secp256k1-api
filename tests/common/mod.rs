//! Shared integration harness and `KMS_TEST_BACKEND` selection (`mock` | `localstack`).

use kms_secp256k1_api::config::{AwsConfig, AwsCreds, Config, ConfigBuilder, HashType};
use kms_secp256k1_api::constants::DEFAULT_AWS_REGION;
use kms_secp256k1_api::create_app;
use std::env;
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;
use tokio::task::JoinHandle;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestBackend {
    Mock,
    Localstack,
}

#[must_use]
pub fn test_backend() -> TestBackend {
    match env::var("KMS_TEST_BACKEND")
        .unwrap_or_else(|_| "mock".into())
        .to_lowercase()
        .as_str()
    {
        "localstack" => TestBackend::Localstack,
        _ => TestBackend::Mock,
    }
}

#[must_use]
pub fn is_localstack() -> bool {
    test_backend() == TestBackend::Localstack
}

fn localstack_creds() -> AwsCreds {
    AwsCreds {
        access_key_id: "test".into(),
        secret_access_key: "test".into(),
    }
}

#[must_use]
pub fn localstack_aws_config(hash_type: HashType) -> AwsConfig {
    let endpoint = env::var("AWS_ENDPOINT").unwrap_or_else(|_| "http://127.0.0.1:4566".to_string());
    let region = env::var("AWS_REGION").unwrap_or_else(|_| DEFAULT_AWS_REGION.to_string());
    let creds = localstack_creds();
    AwsConfig {
        region,
        endpoint,
        sign: creds.clone(),
        create: creds.clone(),
        delete: Some(creds.clone()),
        list: Some(creds),
        hash_type,
        use_default_credentials: false,
    }
}

#[must_use]
pub fn config_builder() -> ConfigBuilder {
    match test_backend() {
        TestBackend::Mock => ConfigBuilder::new(),
        TestBackend::Localstack => ConfigBuilder::new()
            .with_testing_mode(false)
            .with_aws_mode(true)
            .with_aws_config(localstack_aws_config(HashType::Sha256)),
    }
}

#[must_use]
pub fn apply_backend(builder: ConfigBuilder) -> ConfigBuilder {
    match test_backend() {
        TestBackend::Mock => builder,
        TestBackend::Localstack => {
            let built = builder.build();
            let hash_type = if built.is_ethereum_mode() {
                HashType::Keccak256
            } else {
                HashType::Sha256
            };
            let mut next = ConfigBuilder::new()
                .with_testing_mode(built.is_testing_mode())
                .with_aws_mode(true)
                .with_delete_mode(built.is_delete_mode())
                .with_list_mode(built.is_list_mode())
                .with_port(built.get_port())
                .with_eth_chain_id(built.get_eth_chain_id())
                .with_aws_config(localstack_aws_config(hash_type));
            if built.is_ethereum_mode() {
                next = next.with_ethereum_mode();
            } else if built.is_cosmos_mode() {
                next = next.with_cosmos_mode();
            } else {
                next = next.with_casper_mode();
            }
            if built.is_testing_mode() {
                next
            } else {
                next.with_testing_mode(false)
            }
        }
    }
}

/// # Panics
///
/// Panics if the listener cannot bind, the server task fails, or readiness never succeeds.
#[must_use]
pub fn start_test_server(
    config: Config,
) -> Pin<Box<dyn Future<Output = (JoinHandle<()>, String)> + Send>> {
    Box::pin(async move {
        let app = create_app(config).await.expect("create_app");
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("Failed to bind test listener");
        let addr = listener
            .local_addr()
            .expect("Failed to read test listener address");
        let base_url = format!("http://{addr}");

        let handle = tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("Test server failed");
        });

        let client = reqwest::Client::new();
        for _ in 0..100 {
            if let Ok(resp) = client.get(format!("{base_url}/")).send().await
                && resp.status().is_success()
            {
                return (handle, base_url);
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        panic!("Test server at {base_url} did not become ready");
    })
}
