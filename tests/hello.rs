mod common;

use kms_secp256k1_api::config::ConfigBuilder;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_hello_returns_200() {
    let config = ConfigBuilder::new()
        .with_testing_mode(false)
        .with_aws_mode(true) // Do not mock the KMS
        .build();
    let (server_handle, base) = common::start_test_server(config).await;

    let resp = reqwest::get(format!("{base}/"))
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
    let config = ConfigBuilder::new()
        .with_testing_mode(true)
        .with_aws_mode(true) // Do not mock the KMS
        .build();
    let (server_handle, base) = common::start_test_server(config).await;

    let resp = reqwest::get(format!("{base}/"))
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
    let config = ConfigBuilder::new()
        .with_testing_mode(false)
        .with_aws_mode(true) // Do not mock the KMS
        .build();
    let (server_handle, base) = common::start_test_server(config).await;

    let resp = reqwest::get(format!("{base}/?message=HelloWorld"))
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
    let config = ConfigBuilder::new()
        .with_testing_mode(true)
        .with_aws_mode(true) // Do not mock the KMS
        .build();
    let (server_handle, base) = common::start_test_server(config).await;

    let resp = reqwest::get(format!("{base}/?message=HelloWorld"))
        .await
        .expect("Failed to send request");

    assert!(resp.status().is_success());

    let body = resp.text().await.expect("Failed to read response body");
    assert!(body.contains("KMS TESTING_MODE"));
    assert!(body.contains("HelloWorld"));
    assert!(body.contains("Version:"));

    server_handle.abort();
}
