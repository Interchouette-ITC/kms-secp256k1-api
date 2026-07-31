#[cfg(all(test, feature = "casper"))]
mod tests {
    use crate::config::ConfigBuilder;
    use crate::create_app;
    use crate::routes::CreateKeyResponse;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use http_body_util::BodyExt;
    use serde_json::Value;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_list_keys_returns_200_and_keys_present() {
        let config = ConfigBuilder::new().with_list_mode(true).build();
        let app = create_app(config).await;

        // Create two keys
        let mut created_keys = Vec::new();
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

            created_keys.push(parsed);
        }

        // Call listKeys
        let response = app
            .oneshot(Request::get("/listKeys").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        let json: Value = serde_json::from_str(&body_str).unwrap();
        let keys = json["keys"]
            .as_array()
            .expect("Expected 'keys' to be an array in the response");

        assert_eq!(
            keys.len(),
            created_keys.len(),
            "Mismatch in number of returned keys"
        );

        for created in &created_keys {
            let matching = keys.iter().find(|key| {
                key["address"].as_str() == Some(&created.address)
                    && key["key_id"].as_str().is_some()
                    && !key["key_id"].as_str().unwrap().is_empty()
            });

            assert!(
                matching.is_some(),
                "Expected address {} to be in the response with a non-empty key_id",
                created.address
            );
        }
    }

    #[tokio::test]
    async fn test_list_keys_returns_404_when_empty() {
        let config = ConfigBuilder::new().with_list_mode(true).build();
        let app = create_app(config).await;

        let response = app
            .oneshot(Request::get("/listKeys").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        assert!(
            body_str.contains("error"),
            "Expected error in response body, got: {body_str}"
        );
    }

    #[tokio::test]
    async fn test_list_keys_returns_404_when_disabled() {
        let config = ConfigBuilder::new().with_list_mode(false).build();
        let app = create_app(config).await;

        let response = app
            .oneshot(Request::get("/listKeys").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
