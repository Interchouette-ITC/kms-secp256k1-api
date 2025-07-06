use kms_secp256k1_api::{
    config::Config,
    constants::ETH_TRANSACTION_HASH,
    routes::{Approval, CreateKeyResponse},
    run_server,
};
use serial_test::serial;
use std::time::Duration;
use tokio::task;

async fn start_server() -> task::JoinHandle<()> {
    let config = Config {
        ethereum_mode: true,
        ..Default::default()
    };

    task::spawn(async move {
        let _ = run_server(config).await;
    })
}

#[tokio::test]
#[serial]
async fn test_sign_eth_transaction_hash_returns_200_integration() {
    let server_handle = start_server().await;
    tokio::time::sleep(Duration::from_secs(1)).await;

    let client = reqwest::Client::new();
    let base_url = "http://127.0.0.1:4000";
    let transaction_hash = ETH_TRANSACTION_HASH;

    let mut public_keys = Vec::new();
    for _ in 0..2 {
        let resp = client
            .post(format!("{base_url}/createKey"))
            .send()
            .await
            .expect("Failed to send createKey");

        assert!(resp.status().is_success());

        let parsed: CreateKeyResponse = resp
            .json()
            .await
            .expect("Failed to parse createKey response");

        public_keys.push(parsed.public_key);
    }

    let query_string = public_keys
        .iter()
        .map(|k| format!("public_keys={k}"))
        .collect::<Vec<_>>()
        .join("&");

    let sign_url = format!("{base_url}/signTransactionHash?{query_string}");

    let sign_resp = client
        .post(&sign_url)
        .header("Content-Type", "text/plain")
        .body(transaction_hash)
        .send()
        .await
        .expect("Failed to send signTransactionHash");

    assert!(sign_resp.status().is_success());

    let approvals: Vec<Approval> = sign_resp
        .json()
        .await
        .expect("Failed to parse signTransactionHash response");

    assert_eq!(approvals.len(), 2, "Expected 2 approvals for 2 public keys");

    for approval in approvals {
        assert!(
            public_keys.contains(&approval.signer),
            "Unexpected signer: {}",
            approval.signer
        );

        assert_eq!(
            approval.signature.len(),
            130,
            "Expected 65-byte signature (130 hex chars), got {}",
            approval.signature.len()
        );
    }

    server_handle.abort();
}
