mod common;

use kms_secp256k1_api::{config::ConfigBuilder, routes::CreateKeyResponse};
use serde_json::Value;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_list_keys_returns_200_and_keys_present() {
    let config = ConfigBuilder::new().with_list_mode(true).build();
    let (server_handle, base) = common::start_test_server(config).await;

    let client = reqwest::Client::new();
    let mut created_keys = Vec::new();

    // Create two keys
    for _ in 0..2 {
        let resp = client
            .post(format!("{base}/createKey"))
            .send()
            .await
            .expect("Failed to create key");

        assert!(resp.status().is_success());

        let parsed: CreateKeyResponse = resp
            .json()
            .await
            .expect("Invalid JSON response during createKey");

        created_keys.push(parsed);
    }

    // Call listKeys
    let resp = client
        .get(format!("{base}/listKeys"))
        .send()
        .await
        .expect("Failed to send request to listKeys");

    assert!(resp.status().is_success());

    let json: Value = resp.json().await.expect("Invalid JSON from listKeys");
    let keys = json["keys"]
        .as_array()
        .expect("Expected 'keys' array in response");

    assert_eq!(
        keys.len(),
        created_keys.len(),
        "Mismatch in number of returned keys"
    );

    for created in &created_keys {
        let matching = keys.iter().find(|key| {
            key["address"].as_str() == Some(&created.address)
                && key["key_id"].as_str().is_some_and(|id| !id.is_empty())
        });

        assert!(
            matching.is_some(),
            "Expected address {} to be in the response with a non-empty key_id",
            created.address
        );
    }

    server_handle.abort();
}

#[tokio::test]
#[serial]
async fn test_list_keys_returns_404_when_empty() {
    let config = ConfigBuilder::new().with_list_mode(true).build();
    let (server_handle, base) = common::start_test_server(config).await;

    // Empty key store returns 404 (not an internal error)
    let resp = reqwest::get(format!("{base}/listKeys"))
        .await
        .expect("Failed to send request to listKeys");

    assert_eq!(resp.status(), reqwest::StatusCode::NOT_FOUND);

    let body = resp.text().await.expect("Failed to read response body");
    assert!(
        body.contains("error"),
        "Expected 'error' in response body, got: {body}"
    );

    server_handle.abort();
}

#[tokio::test]
#[serial]
async fn test_list_keys_returns_404_when_disabled() {
    let config = ConfigBuilder::new().with_list_mode(false).build();
    let (server_handle, base) = common::start_test_server(config).await;

    let resp = reqwest::get(format!("{base}/listKeys"))
        .await
        .expect("Failed to send request to listKeys");

    assert!(resp.status().is_client_error());

    server_handle.abort();
}
