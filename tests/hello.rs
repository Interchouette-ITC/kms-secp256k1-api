use kms_secp256k1_api::{config::ConfigBuilder, run_server};
use serial_test::serial;
use std::time::Duration;
use tokio::task;

async fn start_server(testing_mode: bool) -> task::JoinHandle<()> {
    let config = ConfigBuilder::new()
        .with_testing_mode(testing_mode)
        .with_aws_mode(true) // Do not mock the KMS
        .build();

    task::spawn(async move {
        let _ = run_server(config).await;
    })
}

#[tokio::test]
#[serial]
async fn test_hello_returns_200() {
    let server_handle = start_server(false).await;
    tokio::time::sleep(Duration::from_secs(1)).await;

    let resp = reqwest::get("http://127.0.0.1:4000/")
        .await
        .expect("Failed to send request");

    assert!(resp.status().is_success());

    let body = resp.text().await.expect("Failed to read response body");
    assert!(body.contains("Hello KMS!"));
    assert!(body.contains("Version:"));

    server_handle.abort();
}

#[tokio::test]
#[serial]
async fn test_hello_returns_200_mocked() {
    let server_handle = start_server(true).await;
    tokio::time::sleep(Duration::from_secs(1)).await;

    let resp = reqwest::get("http://127.0.0.1:4000/")
        .await
        .expect("Failed to send request");

    assert!(resp.status().is_success());

    let body = resp.text().await.expect("Failed to read response body");
    assert!(body.contains("KMS TESTING_MODE"));
    assert!(body.contains("Version:"));

    server_handle.abort();
}

#[tokio::test]
#[serial]
async fn test_hello_with_custom_message() {
    let server_handle = start_server(false).await;
    tokio::time::sleep(Duration::from_secs(1)).await;

    let resp = reqwest::get("http://127.0.0.1:4000/?message=HelloWorld")
        .await
        .expect("Failed to send request");

    assert!(resp.status().is_success());

    let body = resp.text().await.expect("Failed to read response body");
    assert!(!body.contains("KMS TESTING_MODE"));
    assert!(body.contains("HelloWorld"));
    assert!(body.contains("Version:"));

    server_handle.abort();
}

#[tokio::test]
#[serial]
async fn test_hello_with_custom_message_mocked() {
    let server_handle = start_server(true).await;
    tokio::time::sleep(Duration::from_secs(1)).await;

    let resp = reqwest::get("http://127.0.0.1:4000/?message=HelloWorld")
        .await
        .expect("Failed to send request");

    assert!(resp.status().is_success());

    let body = resp.text().await.expect("Failed to read response body");
    assert!(body.contains("KMS TESTING_MODE"));
    assert!(body.contains("HelloWorld"));
    assert!(body.contains("Version:"));

    server_handle.abort();
}
