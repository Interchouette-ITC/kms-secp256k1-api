use kms_secp256k1_api::{
    config::ConfigBuilder, constants::CASPER_SECP_PREFIX, routes::CreateKeyResponse, run_server,
};
use serial_test::serial;
use std::time::Duration;
use tokio::task;

async fn start_server(ethereum_mode: bool, cosmos_mode: bool) -> task::JoinHandle<()> {
    let config = if ethereum_mode {
        ConfigBuilder::new().with_ethereum_mode().build()
    } else if cosmos_mode {
        ConfigBuilder::new().with_cosmos_mode().build()
    } else {
        ConfigBuilder::new().with_casper_mode().build()
    };

    task::spawn(async move {
        let _ = run_server(config).await;
    })
}

#[tokio::test]
#[serial]
async fn test_create_ethereum_keypair_returns_201_mocked() {
    let ethereum_mode = true;
    let server_handle = start_server(ethereum_mode, false).await;
    tokio::time::sleep(Duration::from_secs(1)).await;

    let client = reqwest::Client::new();
    let resp = client
        .post("http://127.0.0.1:4000/createKey")
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
        hex.len()
    );

    server_handle.abort();
}

#[tokio::test]
#[serial]
async fn test_create_casper_keypair_returns_201_mocked() {
    let server_handle = start_server(false, false).await;
    tokio::time::sleep(Duration::from_secs(1)).await;

    let client = reqwest::Client::new();
    let resp = client
        .post("http://127.0.0.1:4000/createKey")
        .send()
        .await
        .expect("Failed to send request");

    assert!(resp.status().is_success());

    let parsed: CreateKeyResponse = resp.json().await.expect("Invalid JSON response");
    let hex = parsed.address.clone();

    assert_eq!(
        hex.len(),
        68,
        "Expected 34 bytes (68 hex chars), got {}",
        hex.len()
    );
    assert!(
        hex.starts_with(CASPER_SECP_PREFIX),
        "Expected Secp256k1 prefix (02), got prefix: {}",
        &hex[..2]
    );

    let address = parsed.address;
    assert!(
        address.eq(&hex),
        "Expected casper address to be public key hex, got: {}",
        &address
    );

    server_handle.abort();
}

#[tokio::test]
#[serial]
async fn test_create_cosmos_keypair_returns_201_mocked() {
    let is_cosmos_mode = true;
    let server_handle = start_server(false, is_cosmos_mode).await;
    tokio::time::sleep(Duration::from_secs(1)).await;

    let client = reqwest::Client::new();
    let resp = client
        .post("http://127.0.0.1:4000/createKey")
        .send()
        .await
        .expect("Failed to send request");

    assert!(resp.status().is_success());

    let parsed: CreateKeyResponse = resp.json().await.expect("Invalid JSON response");

    let hex = parsed.public_key;

    assert_eq!(hex.len(), 66, "Expected 66 hex chars, got {}", hex.len());

    assert!(
        hex.starts_with("02") || hex.starts_with("03"),
        "Expected Secp256k1 compressed key prefix (02 or 03), got: {}",
        &hex[..2]
    );

    let address = parsed.address.trim_start_matches("0x");
    assert!(
        address.starts_with("cosmos1"),
        "Expected cosmos address prefix (cosmos1), got: {}",
        &address[..7]
    );

    server_handle.abort();
}
