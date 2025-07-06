#[cfg(test)]
mod tests {
    use crate::create_app;
    use crate::routes::CreateKeyResponse;
    use crate::{config::Config, constants::ETH_TRANSACTION_HASH};
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_eth_transaction_hash_returns_200() {
        let config = Config {
            ethereum_mode: true,
            ..Default::default()
        };
        let app = create_app(config).await;

        let transaction_hash = ETH_TRANSACTION_HASH;

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

            public_keys.push(parsed.public_key);
        }

        let query_string = public_keys
            .iter()
            .map(|key| format!("public_keys={key}"))
            .collect::<Vec<_>>()
            .join("&");

        let uri = format!("/signTransactionHash?{query_string}");
        let response = app
            .oneshot(
                Request::post(&uri)
                    .header("content-type", "text/plain")
                    .body(Body::from(transaction_hash))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        let parsed_json: serde_json::Value = serde_json::from_str(&body_str).unwrap();
        let approvals = parsed_json.as_array().expect("Expected JSON array");

        for approval in approvals {
            let signer = approval
                .get("signer")
                .and_then(|v| v.as_str())
                .expect("Missing signer field");
            let signature = approval
                .get("signature")
                .and_then(|v| v.as_str())
                .expect("Missing signature field");

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
