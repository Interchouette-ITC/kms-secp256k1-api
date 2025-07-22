#[cfg(test)]
mod tests {
    use crate::config::ConfigBuilder;
    use crate::create_app;
    use crate::routes::CreateKeyResponse;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use casper_rust_wasm_sdk::{
        SDK,
        types::{
            transaction::Transaction,
            transaction_params::transaction_str_params::TransactionStrParams,
        },
    };
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_casper_transaction_hash_returns_200() {
        let config = ConfigBuilder::new().with_casper_mode().build();
        let app = create_app(config).await;

        let mut public_keys = Vec::new();
        for _ in 0..2 {
            let response = app
                .clone()
                .oneshot(Request::post("/createKey").body(Body::empty()).unwrap())
                .await
                .unwrap();

            assert_eq!(response.status(), StatusCode::CREATED);

            let body = response.into_body().collect().await.unwrap().to_bytes();
            let body_str = String::from_utf8(body.to_vec()).unwrap();
            let parsed: CreateKeyResponse = serde_json::from_str(&body_str).unwrap();

            public_keys.push(parsed.address);
        }

        let query_string = public_keys
            .iter()
            .map(|key| format!("public_keys={key}"))
            .collect::<Vec<_>>()
            .join("&");
        dbg!(public_keys.first().unwrap());
        let transaction_params = TransactionStrParams::default();
        transaction_params.set_chain_name("casper-net-1");
        transaction_params.set_initiator_addr(public_keys.first().unwrap());
        transaction_params.set_payment_amount("100000000");
        let sdk = SDK::new(None, None, None);
        let transaction = sdk
            .make_transfer_transaction(
                None,
                public_keys.last().unwrap(),
                "2500000000",
                transaction_params,
                None,
            )
            .map_err(|e| format!("Failed to create transfer transaction: {e}"))
            .unwrap();

        let transaction_str = transaction.to_json_string().unwrap_or_default();

        let uri = format!("/signTransaction?{query_string}");
        let response = app
            .oneshot(
                Request::post(&uri)
                    .header("content-type", "application/json")
                    .body(Body::from(transaction_str))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        for key in &public_keys {
            assert!(
                body_str.contains(key),
                "Response does not contain key: {key}"
            );
        }

        let parsed_transaction: Transaction =
            serde_json::from_str(&body_str).expect("Failed to deserialize response to Transaction");
        let approvals = parsed_transaction.approvals();

        for approval in approvals {
            let signer = approval.signer().to_hex_string();
            let signature = approval.signature().to_hex_string();

            assert!(
                public_keys.contains(&signer.to_string()),
                "Unexpected signer: {signer}"
            );

            assert_eq!(
                signature.len(),
                130,
                "Signature length incorrect for signer {signer}: {}",
                signature.len()
            );
        }
    }
}
