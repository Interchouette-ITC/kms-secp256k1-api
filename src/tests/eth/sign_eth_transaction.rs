#[cfg(test)]
mod tests {
    use crate::constants::ETH_TRANSACTION;
    use crate::create_app;
    use crate::routes::CreateKeyResponse;
    use crate::{config::ConfigBuilder, constants::SIGNATURE_RSV_LEN};
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use http_body_util::BodyExt;
    use serde_json::Value;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_eth_transaction_hash_returns_200() {
        let config = ConfigBuilder::new().with_ethereum_mode().build();

        let app = create_app(config).await.expect("create_app");

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
            .map(|key| format!("keys={key}"))
            .collect::<Vec<_>>()
            .join("&");

        let uri = format!("/signTransaction?{query_string}");
        let response = app
            .oneshot(
                Request::post(&uri)
                    .header("content-type", "application/json")
                    .body(Body::from(ETH_TRANSACTION))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        let parsed: Value = serde_json::from_str(&body_str).expect("Invalid response JSON");

        // Check if all keys are present in the response
        for key in &public_keys {
            assert!(
                body_str.contains(key),
                "Response does not contain key: {key}"
            );
        }

        let signatures = parsed
            .get("signatures")
            .and_then(|v| v.as_array())
            .expect("Missing or invalid 'signatures' array");

        assert_eq!(signatures.len(), public_keys.len());

        for sig in signatures {
            let signer = sig
                .get("signer")
                .and_then(|v| v.as_str())
                .expect("Missing 'signer' in signature");

            let signature = sig
                .get("signature")
                .and_then(|v| v.as_str())
                .expect("Missing 'signature' in signature");

            assert!(
                public_keys.iter().any(|k| k.as_str() == signer),
                "Unexpected signer: {signer}"
            );

            assert_eq!(
                signature.strip_prefix("0x").unwrap_or(signature).len(),
                SIGNATURE_RSV_LEN,
                "Signature length incorrect for signer {signer}: {}",
                signature.len()
            );
        }
    }
}
