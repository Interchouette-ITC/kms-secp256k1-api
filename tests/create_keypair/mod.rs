use kms_secp256k1_api::{constants::CASPER_SECP_PREFIX, routes::CreateKeyResponse};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_create_ethereum_keypair_returns_201_mocked() {
    let config = crate::common::config_builder().with_ethereum_mode().build();
    let (server_handle, base) = crate::common::start_test_server(config).await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{base}/createKey"))
        .send()
        .await
        .expect("Failed to send request");

    assert!(resp.status().is_success());

    let parsed: CreateKeyResponse = resp.json().await.expect("Invalid JSON response");
    let hex = parsed.public_key;

    assert_eq!(
        hex.len(),
        66,
        "Expected 33 bytes (66 hex chars), got {}",
        hex.len()
    );
    assert!(
        hex.starts_with("02") || hex.starts_with("03"),
        "Expected compressed Secp256k1 public key prefix (02 or 03), got: {}",
        &hex[..2]
    );

    let address = parsed.address.trim_start_matches("0x");
    assert_eq!(
        address.len(),
        40,
        "Expected 20 bytes (40 hex chars), got {}",
        address.len()
    );

    server_handle.abort();
}

#[tokio::test]
#[serial]
async fn test_create_casper_keypair_returns_201_mocked() {
    let config = crate::common::config_builder().with_casper_mode().build();
    let (server_handle, base) = crate::common::start_test_server(config).await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{base}/createKey"))
        .send()
        .await
        .expect("Failed to send request");

    assert!(resp.status().is_success());

    let parsed: CreateKeyResponse = resp.json().await.expect("Invalid JSON response");
    let hex = parsed.address;

    assert_eq!(
        hex.len(),
        68,
        "Expected Casper-prefixed key (68 hex chars), got {}",
        hex.len()
    );
    assert!(
        hex.starts_with(CASPER_SECP_PREFIX),
        "Expected Casper secp prefix {CASPER_SECP_PREFIX}, got: {}",
        &hex[..2.min(hex.len())]
    );

    server_handle.abort();
}

#[tokio::test]
#[serial]
async fn test_create_cosmos_keypair_returns_201_mocked() {
    let config = crate::common::config_builder().with_cosmos_mode().build();
    let (server_handle, base) = crate::common::start_test_server(config).await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{base}/createKey"))
        .send()
        .await
        .expect("Failed to send request");

    assert!(resp.status().is_success());

    let parsed: CreateKeyResponse = resp.json().await.expect("Invalid JSON response");
    let hex = parsed.public_key;

    assert_eq!(hex.len(), 66, "Expected 66 hex chars, got {}", hex.len());
    assert!(
        hex.starts_with("02") || hex.starts_with("03"),
        "Expected compressed Secp256k1 public key prefix (02 or 03), got: {}",
        &hex[..2]
    );
    assert!(
        !parsed.address.is_empty(),
        "Expected non-empty cosmos address"
    );

    server_handle.abort();
}
