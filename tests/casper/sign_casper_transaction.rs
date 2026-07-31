use casper_rust_wasm_sdk::{
    SDK,
    types::{
        transaction::Transaction, transaction_params::transaction_str_params::TransactionStrParams,
    },
};
use kms_secp256k1_api::{
    config::ConfigBuilder, constants::SIGNATURE_RSV_LEN, routes::CreateKeyResponse,
};
use serial_test::serial;
use std::collections::HashSet;

#[tokio::test]
#[serial]
async fn test_sign_casper_transaction_returns_200_integration() {
    let config = ConfigBuilder::new().build();
    let (server_handle, base_url) = crate::common::start_test_server(config).await;

    let client = reqwest::Client::new();

    let mut public_keys = Vec::new();
    for _ in 0..2 {
        let resp = client
            .post(format!("{base_url}/createKey"))
            .send()
            .await
            .expect("Failed to call /createKey");

        assert!(resp.status().is_success());

        let parsed: CreateKeyResponse = resp.json().await.expect("Invalid createKey response");
        public_keys.push(parsed.address);
    }

    let tx_params = TransactionStrParams::default();
    tx_params.set_chain_name("casper-net-1");
    tx_params.set_initiator_addr(&public_keys[0]);
    tx_params.set_payment_amount("100000000");

    let sdk = SDK::new(None, None, None);
    let transaction = sdk
        .make_transfer_transaction(None, &public_keys[1], "2500000000", tx_params, None)
        .expect("Failed to create transfer transaction");

    let transaction_json = transaction.to_json_string().unwrap();

    let query_string = public_keys
        .iter()
        .map(|k| format!("keys={k}"))
        .collect::<Vec<_>>()
        .join("&");

    let sign_url = format!("{base_url}/signTransaction?{query_string}");

    let sign_resp = client
        .post(&sign_url)
        .header("Content-Type", "application/json")
        .body(transaction_json)
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

    let signed_transaction: Transaction =
        serde_json::from_str(&resp_body).expect("Failed to parse signed transaction");

    let approvals = signed_transaction.approvals();
    let approval_signers: HashSet<_> = approvals
        .iter()
        .map(|a| a.signer().to_hex_string())
        .collect();

    for key in &public_keys {
        assert!(
            approval_signers.contains(key),
            "Approval missing for key: {key}"
        );
    }

    for approval in approvals {
        let sig = approval.signature().to_hex_string();
        assert_eq!(
            sig.len(),
            SIGNATURE_RSV_LEN,
            "Signature length incorrect: expected {}, got {}",
            SIGNATURE_RSV_LEN,
            sig.len()
        );
    }

    server_handle.abort();
}
