use kms_secp256k1_api::{config::ConfigBuilder, routes::CreateKeyResponse, run_server};
use serde_json::Value;
use serial_test::serial;
use std::time::Duration;
use tokio::task;

async fn start_server(list_mode: bool) -> task::JoinHandle<()> {
    let config = ConfigBuilder::new().with_list_mode(list_mode).build();

    task::spawn(async move {
        let _ = run_server(config).await;
    })
}

#[tokio::test]
#[serial]
async fn test_list_keys_returns_200_and_keys_present() {
    let server_handle = start_server(true).await;
    tokio::time::sleep(Duration::from_secs(1)).await;

    let client = reqwest::Client::new();
    let mut created_keys = Vec::new();

    // Create two keys
    for _ in 0..2 {
        let resp = client
            .post("http://127.0.0.1:4000/createKey")
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
        .get("http://127.0.0.1:4000/listKeys")
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
            key["public_key"].as_str() == Some(&created.public_key)
                && key["key_id"].as_str().is_some_and(|id| !id.is_empty())
        });

        assert!(
            matching.is_some(),
            "Expected public_key {} to be in the response with a non-empty key_id",
            created.public_key
        );
    }

    server_handle.abort();
}

#[tokio::test]
#[serial]
async fn test_list_keys_returns_500_on_error() {
    let server_handle = start_server(true).await;
    tokio::time::sleep(Duration::from_secs(1)).await;

    // Simulate internal error by skipping key creation, if that causes a 500 in your implementation
    let resp = reqwest::get("http://127.0.0.1:4000/listKeys")
        .await
        .expect("Failed to send request to listKeys");

    assert!(resp.status().is_server_error());

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
    let server_handle = start_server(false).await;
    tokio::time::sleep(Duration::from_secs(1)).await;

    let resp = reqwest::get("http://127.0.0.1:4000/listKeys")
        .await
        .expect("Failed to send request to listKeys");

    assert!(resp.status().is_client_error());

    server_handle.abort();
}
