#[cfg(test)]
mod tests {
    use crate::config::Config;
    use crate::create_app;
    use crate::routes::CreateKeyResponse;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_create_ethereum_create_keypair_returns_201() {
        let config = Config {
            ethereum_mode: true,  // ethereum_mode = true
            ..Default::default()  // testing_mode = true
        };
        let app = create_app(config).await;

        let response = app
            .oneshot(Request::post("/createKey").body(Body::empty()).unwrap())
            .await
            .unwrap();

        let status = response.status();
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        assert_eq!(status, StatusCode::CREATED);

        let parsed: CreateKeyResponse = serde_json::from_str(&body_str).unwrap();
        let hex = parsed.public_key.trim_start_matches("0x");

        assert_eq!(
            hex.len(),
            66,
            "Expected 33 bytes (66 hex chars), got {}",
            hex.len()
        );
        assert!(
            hex.starts_with("02") || hex.starts_with("03"),
            "Expected compressed Secp256k1 public key prefix (02 or 03), got: {}",
            &hex[..2]
        );
    }

    #[tokio::test]
    async fn test_create_casper_create_keypair_returns_201() {
        let config = Config {
            casper_mode: true,
            ..Default::default()
        };
        let app = create_app(config).await;

        let response = app
            .oneshot(Request::post("/createKey").body(Body::empty()).unwrap())
            .await
            .unwrap();

        let status = response.status();
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        assert_eq!(status, StatusCode::CREATED);

        let parsed: CreateKeyResponse = serde_json::from_str(&body_str).unwrap();
        let hex = parsed.public_key.trim_start_matches("0x");

        assert_eq!(
            hex.len(),
            68,
            "Expected 34 bytes (68 hex chars), got {}",
            hex.len()
        );
        assert!(
            hex.starts_with("02"),
            "Expected Secp256k1 prefix (02), got prefix: {}",
            &hex[..2]
        );
    }
}
