use kms_secp256k1_api::{config::ConfigBuilder, routes::CreateKeyResponse, run_server};
use serial_test::serial;
use std::time::Duration;
use tokio::task;

async fn start_server(ethereum_mode: bool) -> task::JoinHandle<()> {
    let config = if ethereum_mode {
        ConfigBuilder::new().with_ethereum_mode().build()
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
    let server_handle = start_server(ethereum_mode).await;
    tokio::time::sleep(Duration::from_secs(1)).await;

    let client = reqwest::Client::new();
    let resp = client
        .post("http://127.0.0.1:4000/createKey")
        .send()
        .await
        .expect("Failed to send request");

    assert!(resp.status().is_success());

    let parsed: CreateKeyResponse = resp.json().await.expect("Invalid JSON response");
    let hex = parsed.public_key.trim_start_matches("0x");

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

    server_handle.abort();
}

#[tokio::test]
#[serial]
async fn test_create_casper_keypair_returns_201_mocked() {
    let server_handle = start_server(false).await;
    tokio::time::sleep(Duration::from_secs(1)).await;

    let client = reqwest::Client::new();
    let resp = client
        .post("http://127.0.0.1:4000/createKey")
        .send()
        .await
        .expect("Failed to send request");

    assert!(resp.status().is_success());

    let parsed: CreateKeyResponse = resp.json().await.expect("Invalid JSON response");
    let hex = parsed.public_key.trim_start_matches("0x");

    assert_eq!(
        hex.len(),
        68,
        "Expected 34 bytes (68 hex chars), got {}",
        hex.len()
    );
    assert!(
        hex.starts_with("02"),
        "Expected Secp256k1 prefix (02), got prefix: {}",
        &hex[..2]
    );

    server_handle.abort();
}
