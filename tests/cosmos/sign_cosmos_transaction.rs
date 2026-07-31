use kms_secp256k1_api::{
    constants::{COSMOS_TRANSACTION, SIGNATURE_RS_LEN},
    routes::CreateKeyResponse,
};
use serde_json::Value;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_sign_cosmos_transaction_returns_200_integration() {
    let config = crate::common::config_builder().with_cosmos_mode().build();
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
        .body(COSMOS_TRANSACTION)
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
            SIGNATURE_RS_LEN,
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
