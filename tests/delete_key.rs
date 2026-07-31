mod common;

use kms_secp256k1_api::config::ConfigBuilder;
use kms_secp256k1_api::{constants::CASPER_PUBLIC_KEY_PREFIXED, routes::CreateKeyResponse};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_delete_key_returns_200_when_key_exists() {
    let config = ConfigBuilder::new().with_delete_mode(true).build();
    let (server_handle, base) = common::start_test_server(config).await;

    let client = reqwest::Client::new();

    // Create a key first
    let create_resp = client
        .post(format!("{base}/createKey"))
        .send()
        .await
        .expect("Failed to send createKey request");

    assert!(create_resp.status().is_success());

    let created: CreateKeyResponse = create_resp
        .json()
        .await
        .expect("Failed to parse createKey response");

    // Delete the created key
    let url = format!("{base}/deleteKey?key={}", created.address);
    let delete_resp = client
        .delete(&url)
        .send()
        .await
        .expect("Failed to send deleteKey request");

    assert!(delete_resp.status().is_success());

    let body = delete_resp.text().await.expect("Failed to read body");

    assert!(
        body.contains("\"deleted\":true"),
        "Expected successful deletion, got body: {body}"
    );

    server_handle.abort();
}

#[tokio::test]
#[serial]
async fn test_delete_key_returns_200_when_key_does_not_exist() {
    let config = ConfigBuilder::new().with_delete_mode(true).build();
    let (server_handle, base) = common::start_test_server(config).await;

    let client = reqwest::Client::new();

    let fake_key = CASPER_PUBLIC_KEY_PREFIXED;
    let url = format!("{base}/deleteKey?key={fake_key}");
    let resp = client
        .delete(&url)
        .send()
        .await
        .expect("Failed to send deleteKey request");

    assert!(resp.status().is_success());

    let body = resp.text().await.expect("Failed to read body");

    assert!(
        body.contains("\"deleted\":false"),
        "Expected deleted:false for non-existent key, got: {body}"
    );

    server_handle.abort();
}

#[tokio::test]
#[serial]
async fn test_delete_key_returns_404_when_disabled() {
    let config = ConfigBuilder::new().with_delete_mode(false).build();
    let (server_handle, base) = common::start_test_server(config).await;

    let client = reqwest::Client::new();

    let url = format!("{base}/deleteKey?key=dummykey");
    let resp = client
        .delete(&url)
        .send()
        .await
        .expect("Failed to send deleteKey request");

    assert!(resp.status().is_client_error());

    server_handle.abort();
}

#[tokio::test]
#[serial]
async fn test_delete_key_returns_200_when_key_exists_from_address() {
    let config = ConfigBuilder::new().with_delete_mode(true).build();
    let (server_handle, base) = common::start_test_server(config).await;

    let client = reqwest::Client::new();

    // Create a key first
    let create_resp = client
        .post(format!("{base}/createKey"))
        .send()
        .await
        .expect("Failed to send createKey request");

    assert!(create_resp.status().is_success());

    let created: CreateKeyResponse = create_resp
        .json()
        .await
        .expect("Failed to parse createKey response");

    // Delete the created key
    let url = format!("{base}/deleteKey?key={}", created.address);
    let delete_resp = client
        .delete(&url)
        .send()
        .await
        .expect("Failed to send deleteKey request");

    assert!(delete_resp.status().is_success());

    let body = delete_resp.text().await.expect("Failed to read body");

    assert!(
        body.contains("\"deleted\":true"),
        "Expected successful deletion, got body: {body}"
    );

    server_handle.abort();
}

#[tokio::test]
#[serial]
async fn test_delete_key_returns_200_when_key_does_not_exist_from_address() {
    let config = ConfigBuilder::new().with_delete_mode(true).build();
    let (server_handle, base) = common::start_test_server(config).await;

    let client = reqwest::Client::new();

    let fake_key = "fake_address";
    let url = format!("{base}/deleteKey?key={fake_key}");
    let resp = client
        .delete(&url)
        .send()
        .await
        .expect("Failed to send deleteKey request");

    assert!(resp.status().is_success());

    let body = resp.text().await.expect("Failed to read body");

    assert!(
        body.contains("\"deleted\":false"),
        "Expected deleted:false for non-existent key, got: {body}"
    );

    server_handle.abort();
}

#[tokio::test]
#[serial]
async fn test_delete_eth_key_returns_200_when_key_exists() {
    let config = ConfigBuilder::new()
        .with_delete_mode(true)
        .with_ethereum_mode()
        .build();
    let (server_handle, base) = common::start_test_server(config).await;

    let client = reqwest::Client::new();

    // Create a key first
    let create_resp = client
        .post(format!("{base}/createKey"))
        .send()
        .await
        .expect("Failed to send createKey request");

    assert!(create_resp.status().is_success());

    let created: CreateKeyResponse = create_resp
        .json()
        .await
        .expect("Failed to parse createKey response");

    // Delete the created key
    let url = format!("{base}/deleteKey?key={}", created.public_key);
    let delete_resp = client
        .delete(&url)
        .send()
        .await
        .expect("Failed to send deleteKey request");

    assert!(delete_resp.status().is_success());

    let body = delete_resp.text().await.expect("Failed to read body");

    assert!(
        body.contains("\"deleted\":true"),
        "Expected successful deletion, got body: {body}"
    );

    server_handle.abort();
}

#[tokio::test]
#[serial]
async fn test_delete_eth_key_returns_200_when_key_exists_from_address() {
    let config = ConfigBuilder::new()
        .with_delete_mode(true)
        .with_ethereum_mode()
        .build();
    let (server_handle, base) = common::start_test_server(config).await;

    let client = reqwest::Client::new();

    // Create a key first
    let create_resp = client
        .post(format!("{base}/createKey"))
        .send()
        .await
        .expect("Failed to send createKey request");

    assert!(create_resp.status().is_success());

    let created: CreateKeyResponse = create_resp
        .json()
        .await
        .expect("Failed to parse createKey response");

    // Delete the created key
    let url = format!("{base}/deleteKey?key={}", created.address);
    let delete_resp = client
        .delete(&url)
        .send()
        .await
        .expect("Failed to send deleteKey request");

    assert!(delete_resp.status().is_success());

    let body = delete_resp.text().await.expect("Failed to read body");

    assert!(
        body.contains("\"deleted\":true"),
        "Expected successful deletion, got body: {body}"
    );

    server_handle.abort();
}
