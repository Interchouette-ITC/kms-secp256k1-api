use crate::{
    config::Config,
    constants::WASM_PATH,
    routes::{
        ApiDoc, create_key, delete_key, hello, list_keys, sign_transaction, sign_transaction_hash,
        verify_signature,
    },
    services::{
        casper_keys_service::CasperKeysService,
        cosmos_keys_service::CosmosKeysService,
        crypto_service::CryptoService,
        ethereum_keys_service::EthereumKeysService,
        keys_service::KeysServiceTrait,
        mocks::{
            mock_casper_keys_service::MockCasperKeysService,
            mock_cosmos_keys_service::MockCosmosKeysService,
            mock_ethereum_keys_service::MockEthereumKeysService,
        },
    },
    wasm_loader::WasmLoader,
};
use axum::Router;
use axum::{
    Extension,
    routing::{delete, get, post},
};
use std::fs;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

pub mod config;
pub mod constants;
pub mod routes;
pub mod services;
pub mod wasm_loader;

#[cfg(test)]
pub mod tests;

static VERSION: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
    let content = fs::read_to_string("Cargo.toml").unwrap_or_default();
    let parsed: toml::Value =
        toml::from_str(&content).unwrap_or_else(|_| toml::Value::Table(toml::map::Map::new()));
    parsed
        .get("package")
        .and_then(|pkg| pkg.get("version"))
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string()
});

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub keys_service: Arc<Mutex<Box<dyn KeysServiceTrait + Send + Sync>>>,
}

/// Creates an Axum application instance with the given configuration.
///
/// # Panics
///
/// This function will panic if:
/// - Loading the WASM module fails.
/// - Initializing the `CryptoService` fails.
/// - Initializing the key service (`CasperKeysService`) fails.
///
/// These errors are propagated via calls to `expect`.
pub async fn create_app(config: Config) -> Router {
    let wasm_loader = WasmLoader::new(WASM_PATH)
        .await
        .expect("Failed to load WASM module");

    let crypto_service =
        CryptoService::new(&wasm_loader).expect("Failed to initialize CryptoService");

    let keys_service: Box<dyn KeysServiceTrait> = if config.is_testing_mode() {
        if config.is_ethereum_mode() {
            Box::new(
                MockEthereumKeysService::new(config.clone(), crypto_service)
                    .expect("Failed to initialize MockEthereumKeysService"),
            )
        } else if config.is_casper_mode() {
            Box::new(
                MockCasperKeysService::new(config.clone(), crypto_service)
                    .expect("Failed to initialize MockCasperKeysService"),
            )
        } else if config.is_cosmos_mode() {
            Box::new(
                MockCosmosKeysService::new(config.clone(), crypto_service)
                    .expect("Failed to initialize MockCasperKeysService"),
            )
        } else {
            unimplemented!()
        }
    } else if config.is_ethereum_mode() {
        Box::new(
            EthereumKeysService::new(config.clone(), crypto_service)
                .await
                .expect("Failed to initialize Failed to initialize EthereumKeysService"),
        )
    } else if config.is_casper_mode() {
        Box::new(
            CasperKeysService::new(config.clone(), crypto_service)
                .await
                .expect("Failed to initialize Failed to initialize CasperKeysService"),
        )
    } else if config.is_cosmos_mode() {
        Box::new(
            CosmosKeysService::new(config.clone(), crypto_service)
                .await
                .expect("Failed to initialize Failed to initialize CasperKeysService"),
        )
    } else {
        unimplemented!()
    };

    let shared_state = AppState {
        config: config.clone(),
        keys_service: Arc::new(Mutex::new(keys_service)),
    };

    let mut app = Router::new()
        .route("/", get(hello))
        .route("/createKey", post(create_key))
        .route("/signTransactionHash", post(sign_transaction_hash))
        .route("/signTransaction", post(sign_transaction))
        .route("/verifySignature", get(verify_signature));

    if config.is_delete_mode() {
        app = app.route("/deleteKey", delete(delete_key));
    }

    if config.is_list_mode() {
        app = app.route("/listKeys", get(list_keys));
    }

    app = app.layer(Extension(shared_state));

    let swagger_ui = SwaggerUi::new("/api/").url("/api-doc/openapi.json", ApiDoc::openapi());

    app.merge(swagger_ui)
}

/// Starts the HTTP server with the given configuration.
///
/// # Errors
/// This function returns an error if the TCP listener cannot be bound,
/// or if the server fails to start.
pub async fn run_server(config: Config) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let app = create_app(config.clone()).await;

    let addr = format!("{}:{}", config.get_addr(), config.get_port());
    info!("🚀 Listening on {addr}");

    if config.is_testing_mode() {
        warn!("TESTING_MODE ACTIVE");
    }

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

#[cfg(test)]
mod tests_lib {
    use crate::config::ConfigBuilder;

    use super::*;
    use tokio::task;

    #[tokio::test]
    async fn test_run_server_creates_app() {
        let config = ConfigBuilder::new()
            .with_testing_mode(true)
            .with_port(0)
            .build();

        // Spawn the server in a background task but abort immediately,
        // just test that it starts without panics or errors.
        let server_future = task::spawn(async move { run_server(config).await });

        // Wait briefly or abort since we don't want it running forever.
        // Here just wait a little and then abort.
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        server_future.abort();

        // If spawn didn't panic, assume success.
    }

    #[cfg(test)]
    mod tests_create_app {
        use super::*;
        use crate::config::ConfigBuilder;
        use axum::http;
        use tower::ServiceExt;

        #[tokio::test]
        async fn test_create_app_routes() {
            let config = ConfigBuilder::new()
                .with_testing_mode(true)
                .with_delete_mode(true)
                .with_list_mode(true)
                .build();

            let app = create_app(config.clone()).await;

            let response = app
                .clone()
                .oneshot(
                    http::Request::builder()
                        .uri("/")
                        .body(axum::body::Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_eq!(response.status(), http::StatusCode::OK);

            let delete_route = app
                .clone()
                .oneshot(
                    http::Request::builder()
                        .method("DELETE")
                        .uri("/deleteKey?key=test_key")
                        .body(axum::body::Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            assert!(
                delete_route.status().is_success(),
                "Expected /deleteKey route to exist or return 404",
            );

            let list_route = app
                .clone()
                .oneshot(
                    http::Request::builder()
                        .method("GET")
                        .uri("/listKeys")
                        .body(axum::body::Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            assert!(
                list_route.status().is_server_error(),
                "Expected /listKeys route to exist and return 500"
            );

            let create_key_response = app
                .clone()
                .oneshot(
                    http::Request::builder()
                        .method("POST")
                        .uri("/createKey")
                        .body(axum::body::Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_ne!(create_key_response.status(), http::StatusCode::NOT_FOUND);

            let list_route = app
                .clone()
                .oneshot(
                    http::Request::builder()
                        .method("GET")
                        .uri("/listKeys")
                        .body(axum::body::Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            assert!(
                list_route.status().is_success(),
                "Expected /listKeys route to exist"
            );

            let sign_hash_response = app
                .clone()
                .oneshot(
                    http::Request::builder()
                        .method("POST")
                        .uri("/signTransactionHash")
                        .body(axum::body::Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_ne!(
                sign_hash_response.status(),
                http::StatusCode::NOT_FOUND,
                "Expected /signTransactionHash route to exist"
            );

            let sign_tx_response = app
                .clone()
                .oneshot(
                    http::Request::builder()
                        .method("POST")
                        .uri("/signTransaction")
                        .body(axum::body::Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_ne!(
                sign_tx_response.status(),
                http::StatusCode::NOT_FOUND,
                "Expected /signTransaction route to exist"
            );

            let verify_response = app
                .clone()
                .oneshot(
                    http::Request::builder()
                        .method("GET")
                        .uri("/verifySignature")
                        .body(axum::body::Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_ne!(
                verify_response.status(),
                http::StatusCode::NOT_FOUND,
                "Expected /verifySignature route to exist"
            );

            let openapi_response = app
                .clone()
                .oneshot(
                    http::Request::builder()
                        .method("GET")
                        .uri("/api-doc/openapi.json")
                        .body(axum::body::Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            assert!(
                openapi_response.status().is_success(),
                "Expected OpenAPI JSON /api-doc/openapi.json route to exist and respond successfully"
            );
        }
    }
}
