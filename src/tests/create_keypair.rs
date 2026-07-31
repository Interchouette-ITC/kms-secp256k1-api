#[cfg(test)]
mod tests {
    use crate::create_app;
    use crate::routes::CreateKeyResponse;
    use crate::{config::ConfigBuilder, constants::CASPER_SECP_PREFIX};
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_create_ethereum_create_keypair_returns_201() {
        let config = ConfigBuilder::new().with_ethereum_mode().build();
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
        let hex = parsed.public_key;

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

        let address = parsed.address.trim_start_matches("0x");
        assert_eq!(
            address.len(),
            40,
            "Expected 20 bytes (40 hex chars), got {}",
            hex.len()
        );
    }

    #[tokio::test]
    async fn test_create_casper_create_keypair_returns_201() {
        let config = ConfigBuilder::new().with_casper_mode().build();

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
        let hex = parsed.public_key;

        assert_eq!(
            hex.len(),
            66,
            "Expected 33 bytes (66 hex chars), got {}",
            hex.len()
        );

        let hex = parsed.address.clone();

        assert_eq!(
            hex.len(),
            68,
            "Expected 34 bytes (68 hex chars), got {}",
            hex.len()
        );
        assert!(
            hex.starts_with(CASPER_SECP_PREFIX),
            "Expected Secp256k1 prefix (02), got prefix: {}",
            &hex[..2]
        );

        let address = parsed.address;
        assert!(
            address.eq(&hex),
            "Expected casper address to be public key hex, got: {address}"
        );
    }

    #[tokio::test]
    async fn test_create_cosmos_create_keypair_returns_201() {
        let config = ConfigBuilder::new().with_cosmos_mode().build();

        let app = create_app(config.clone()).await;

        let response = app
            .oneshot(Request::post("/createKey").body(Body::empty()).unwrap())
            .await
            .unwrap();

        let status = response.status();
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        assert_eq!(status, StatusCode::CREATED);

        let parsed: CreateKeyResponse = serde_json::from_str(&body_str).unwrap();
        let hex = parsed.public_key;

        assert_eq!(hex.len(), 66, "Expected 66 hex chars, got {}", hex.len());

        assert!(
            hex.starts_with("02") || hex.starts_with("03"),
            "Expected Secp256k1 compressed key prefix (02 or 03), got: {}",
            &hex[..2]
        );

        let address = parsed.address;
        let address_prefix = &address[..7];
        assert!(
            address.starts_with("cosmos1"),
            "Expected cosmos address prefix (cosmos1), got: {address_prefix}"
        );
    }
}
