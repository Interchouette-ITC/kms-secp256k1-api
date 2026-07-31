#[cfg(test)]
mod tests {
    use crate::config::ConfigBuilder;
    use crate::create_app;
    use crate::routes::CreateKeyResponse;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    #[cfg(feature = "casper")]
    #[tokio::test]
    async fn test_delete_key_returns_200_when_key_exists() {
        let config = ConfigBuilder::new().with_delete_mode(true).build();
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

        let public_key = parsed.address;

        // Delete the key
        let uri = format!("/deleteKey?key={public_key}");
        let delete_response = app
            .oneshot(Request::delete(&uri).body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(delete_response.status(), StatusCode::OK);

        let body = delete_response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes();
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        assert!(
            body_str.contains("\"deleted\":true"),
            "Expected deletion to succeed, got: {body_str}"
        );
    }

    #[cfg(feature = "casper")]
    #[tokio::test]
    async fn test_delete_key_returns_200_when_key_does_not_exist() {
        use crate::constants::CASPER_PUBLIC_KEY_PREFIXED;

        let config = ConfigBuilder::new().with_delete_mode(true).build();
        let app = create_app(config).await;

        let fake_key = CASPER_PUBLIC_KEY_PREFIXED;

        let uri = format!("/deleteKey?key={fake_key}");
        let response = app
            .oneshot(Request::delete(&uri).body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        assert!(
            body_str.contains("\"deleted\":false"),
            "Expected deleted:false for non-existent key, got: {body_str}"
        );
    }

    #[cfg(feature = "casper")]
    #[tokio::test]
    async fn test_delete_key_returns_404_when_disabled() {
        let config = ConfigBuilder::new().with_delete_mode(false).build();
        let app = create_app(config).await;

        let uri = "/deleteKey?key=somekey";
        let response = app
            .oneshot(Request::delete(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[cfg(feature = "ethereum")]
    #[tokio::test]
    async fn test_delete_key_returns_200_when_key_exists_from_address() {
        let config = ConfigBuilder::new()
            .with_delete_mode(true)
            .with_ethereum_mode()
            .build();
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

        let address = parsed.address;

        // Delete the key
        let uri = format!("/deleteKey?key={address}");
        let delete_response = app
            .oneshot(Request::delete(&uri).body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(delete_response.status(), StatusCode::OK);

        let body = delete_response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes();
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        assert!(
            body_str.contains("\"deleted\":true"),
            "Expected deletion to succeed, got: {body_str}"
        );
    }

    #[cfg(feature = "ethereum")]
    #[tokio::test]
    async fn test_delete_key_returns_200_when_key_does_not_exist_from_address() {
        let config = ConfigBuilder::new()
            .with_delete_mode(true)
            .with_ethereum_mode()
            .build();
        let app = create_app(config).await;

        let fake_key = "fake_address";

        let uri = format!("/deleteKey?key={fake_key}");
        let response = app
            .oneshot(Request::delete(&uri).body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        assert!(
            body_str.contains("\"deleted\":false"),
            "Expected deleted:false for non-existent key, got: {body_str}"
        );
    }
}
