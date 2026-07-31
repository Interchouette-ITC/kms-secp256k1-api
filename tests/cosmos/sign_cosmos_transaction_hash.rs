use kms_secp256k1_api::{
    config::ConfigBuilder,
    constants::{COSMOS_TRANSACTION_HASH, SIGNATURE_RS_LEN},
    routes::{Approval, CreateKeyResponse},
};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_sign_eth_transaction_hash_returns_200_integration() {
    let config = ConfigBuilder::new().with_cosmos_mode().build();
    let (server_handle, base_url) = crate::common::start_test_server(config).await;

    let client = reqwest::Client::new();
    let transaction_hash = COSMOS_TRANSACTION_HASH;

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

        public_keys.push(parsed.address);
    }

    let query_string = public_keys
        .iter()
        .map(|k| format!("keys={k}"))
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
            public_keys.contains(&approval.address),
            "Unexpected signer: {}",
            approval.address
        );

        assert_eq!(
            approval.signature.len(),
            SIGNATURE_RS_LEN,
            "Expected 65-byte signature ({} hex chars), got {}",
            SIGNATURE_RS_LEN,
            approval.signature.len()
        );
    }

    server_handle.abort();
}
