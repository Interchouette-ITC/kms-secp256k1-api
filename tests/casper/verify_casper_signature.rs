use kms_secp256k1_api::{
    config::Config,
    constants::TRANSACTION_HASH,
    routes::{Approval, CreateKeyResponse},
    run_server,
};
use serial_test::serial;
use std::time::Duration;
use tokio::task;

async fn start_server() -> task::JoinHandle<()> {
    let config = Config {
        ..Default::default()
    };

    task::spawn(async move {
        let _ = run_server(config).await;
    })
}

async fn run_verify_casper_signature_test(via_kms: bool) {
    let server_handle = start_server().await;
    tokio::time::sleep(Duration::from_secs(1)).await;

    let client = reqwest::Client::new();
    let base_url = "http://127.0.0.1:4000";

    let create_resp = client
        .post(format!("{base_url}/createKey"))
        .send()
        .await
        .expect("Failed to create key");

    assert!(create_resp.status().is_success());

    let created: CreateKeyResponse = create_resp
        .json()
        .await
        .expect("Failed to parse createKey response");

    let public_key = created.public_key;
    let transaction_hash = TRANSACTION_HASH;

    let sign_url = format!("{base_url}/signTransactionHash?public_keys={public_key}");

    let sign_resp = client
        .post(&sign_url)
        .header("Content-Type", "text/plain")
        .body(transaction_hash)
        .send()
        .await
        .expect("Failed to send signTransactionHash request");

    assert!(sign_resp.status().is_success());

    let approvals: Vec<Approval> = sign_resp
        .json()
        .await
        .expect("Failed to parse approval response");

    assert_eq!(approvals.len(), 1);

    let signature = &approvals[0].signature;

    let verify_url = format!(
        "{base_url}/verifySignature?public_key={public_key}&transaction_hash={transaction_hash}&signature={signature}&via_kms={via_kms}"
    );

    let verify_resp = client
        .get(&verify_url)
        .send()
        .await
        .expect("Failed to send verifySignature request");

    assert!(verify_resp.status().is_success());

    let body = verify_resp
        .text()
        .await
        .expect("Failed to read response body");

    assert!(
        body.contains("\"valid\":true"),
        "Expected valid:true, got: {body}"
    );

    server_handle.abort();
}

#[tokio::test]
#[serial]
async fn test_verify_casper_signature_returns_200_integration() {
    run_verify_casper_signature_test(false).await;
}

#[tokio::test]
#[serial]
async fn test_verify_casper_via_kms_signature_returns_200_integration() {
    run_verify_casper_signature_test(true).await;
}
