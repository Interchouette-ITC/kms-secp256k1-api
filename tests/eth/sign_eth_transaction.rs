use kms_secp256k1_api::{
    config::ConfigBuilder, constants::ETH_TRANSACTION, routes::CreateKeyResponse, run_server,
};
use serde_json::Value;
use serial_test::serial;
use std::time::Duration;
use tokio::task;

async fn start_server() -> task::JoinHandle<()> {
    let config = ConfigBuilder::new().with_ethereum_mode().build();

    task::spawn(async move {
        let _ = run_server(config).await;
    })
}

#[tokio::test]
#[serial]
async fn test_sign_eth_transaction_returns_200_integration() {
    let server_handle = start_server().await;
    tokio::time::sleep(Duration::from_secs(1)).await;

    let client = reqwest::Client::new();
    let base_url = "http://127.0.0.1:4000";

    let mut public_keys = Vec::new();
    for _ in 0..2 {
        let resp = client
            .post(format!("{base_url}/createKey"))
            .send()
            .await
            .expect("Failed to call /createKey");

        assert!(resp.status().is_success());

        let parsed: CreateKeyResponse = resp.json().await.expect("Invalid createKey response");
        public_keys.push(parsed.public_key);
    }

    let query_string = public_keys
        .iter()
        .map(|k| format!("keys={k}"))
        .collect::<Vec<_>>()
        .join("&");

    let sign_url = format!("{base_url}/signTransaction?{query_string}");

    let sign_resp = client
        .post(&sign_url)
        .header("Content-Type", "application/json")
        .body(ETH_TRANSACTION)
        .send()
        .await
        .expect("Failed to call /signTransaction");

    assert!(sign_resp.status().is_success());

    let resp_body = sign_resp.text().await.expect("Failed to read response");

    for key in &public_keys {
        assert!(
            resp_body.contains(key),
            "Response does not contain key: {key}"
        );
    }

    let signed_transaction: Value =
        serde_json::from_str(&resp_body).expect("Failed to parse signed transaction");

    let signatures = signed_transaction
        .get("signatures")
        .and_then(|v| v.as_array())
        .expect("Missing or invalid 'signatures' array");

    let mut approval_signers = std::collections::HashSet::new();

    for sig in signatures {
        let signer = sig
            .get("signer")
            .and_then(|v| v.as_str())
            .expect("Missing 'signer' in signature");

        let signature = sig
            .get("signature")
            .and_then(|v| v.as_str())
            .expect("Missing 'signature' in signature");

        approval_signers.insert(signer.to_string());

        assert_eq!(
            signature.strip_prefix("0x").unwrap_or(signature).len(),
            130,
            "Signature length incorrect for signer {signer}: {}",
            signature.len()
        );
    }

    for key in &public_keys {
        assert!(
            approval_signers.contains(key),
            "Signature missing for key: {key}"
        );
    }

    server_handle.abort();
}
