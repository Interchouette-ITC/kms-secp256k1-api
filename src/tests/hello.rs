#[cfg(test)]
mod tests {
    use crate::config::ConfigBuilder;
    use crate::create_app;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_hello_returns_200() {
        let config = ConfigBuilder::new().with_testing_mode(true).build();

        let app = create_app(config).await;

        let response = app
            .oneshot(Request::get("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        let status = response.status();
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        assert_eq!(status, StatusCode::OK);
        assert!(body_str.contains("KMS TESTING_MODE"));
        assert!(body_str.contains("Version:"));
    }

    #[tokio::test]
    async fn test_hello_with_custom_message() {
        let config = ConfigBuilder::new()
            .with_testing_mode(false)
            .with_aws_mode(true) // Do not mock the KMS
            .build();

        let app = create_app(config).await;

        let response = app
            .oneshot(
                Request::get("/?message=AxumTest")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let status = response.status();
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        assert_eq!(status, StatusCode::OK);
        assert!(body_str.contains("Hello AxumTest!"));
        assert!(body_str.contains("Version:"));
    }

    #[tokio::test]
    async fn test_hello_without_message_param() {
        let config = ConfigBuilder::new()
            .with_testing_mode(false)
            .with_aws_mode(true) // Do not mock the KMS
            .build();

        let app = create_app(config).await;

        let response = app
            .oneshot(Request::get("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        let status = response.status();
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        assert_eq!(status, StatusCode::OK);
        assert!(body_str.contains("Hello KMS!"));
        assert!(body_str.contains("Version:"));
    }
}
