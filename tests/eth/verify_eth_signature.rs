use kms_secp256k1_api::{
    config::ConfigBuilder,
    constants::ETH_TRANSACTION_HASH,
    routes::{Approval, CreateKeyResponse},
};
use serial_test::serial;

async fn run_verify_eth_signature_test(via_kms: bool) {
    let config = ConfigBuilder::new().with_ethereum_mode().build();
    let (server_handle, base_url) = crate::common::start_test_server(config).await;

    let client = reqwest::Client::new();

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
    let transaction_hash = ETH_TRANSACTION_HASH;

    let sign_url = format!("{base_url}/signTransactionHash?keys={public_key}");

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
        "{base_url}/verifySignature?key={public_key}&transaction_hash={transaction_hash}&signature={signature}&via_kms={via_kms}"
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
async fn test_verify_eth_signature_returns_200_integration() {
    Box::pin(run_verify_eth_signature_test(false)).await;
}

#[tokio::test]
#[serial]
async fn test_verify_eth_via_kms_signature_returns_200_integration() {
    Box::pin(run_verify_eth_signature_test(true)).await;
}
