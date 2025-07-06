#[cfg(test)]
mod tests {
    use crate::constants::TRANSACTION_HASH;
    use crate::create_app;
    use crate::routes::CreateKeyResponse;
    use crate::{config::Config, routes::Approval};
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    async fn test_verify_signature_returns(via_kms: bool) {
        let config = Config {
            casper_mode: true,
            ..Default::default()
        };
        let app = create_app(config).await;

        let response = app
            .clone()
            .oneshot(Request::post("/createKey").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let parsed: CreateKeyResponse = serde_json::from_str(&body_str).unwrap();

        let public_key = parsed.public_key;

        let transaction_hash = TRANSACTION_HASH;
        let query_string = format!("public_keys={public_key}");

        let uri = format!("/signTransactionHash?{query_string}");
        let sign_response = app
            .clone()
            .oneshot(
                Request::post(&uri)
                    .header("content-type", "text/plain")
                    .body(Body::from(transaction_hash))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(sign_response.status(), StatusCode::OK);

        let body = sign_response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let approvals: Vec<Approval> = serde_json::from_str(&body_str).unwrap();

        assert_eq!(approvals.len(), 1);

        let signature = &approvals[0].signature;

        let uri = format!(
            "/verifySignature?public_key={public_key}&transaction_hash={transaction_hash}&signature={signature}&via_kms={via_kms}"
        );

        let verify_response = app
            .oneshot(Request::get(&uri).body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(verify_response.status(), StatusCode::OK);

        let body = verify_response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes();
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        assert!(
            body_str.contains("\"valid\":true"),
            "Expected verification to succeed, got: {body_str}"
        );
    }

    #[tokio::test]
    async fn test_verify_signature_returns_200() {
        let via_kms = false;
        test_verify_signature_returns(via_kms).await
    }

    #[tokio::test]
    async fn test_verify_via_kms_signature_returns_200() {
        let via_kms = true;
        test_verify_signature_returns(via_kms).await
    }
}
