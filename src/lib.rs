#[cfg(not(any(feature = "casper", feature = "ethereum", feature = "cosmos")))]
compile_error!("Enable at least one chain feature: casper, ethereum, cosmos (or --features all)");

use crate::{
    config::Config,
    constants::WASM_PATH,
    routes::{
        ApiDoc, create_key, delete_key, hello, list_keys, sign_transaction, sign_transaction_hash,
        verify_signature,
    },
    services::{crypto_service::CryptoService, keys_service::KeysServiceTrait},
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
pub mod error;
pub mod routes;
pub mod services;
pub mod wasm_loader;

pub use error::{KmsError, Result};

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

async fn create_keys_service(
    config: &Config,
    crypto_service: CryptoService,
) -> Box<dyn KeysServiceTrait + Send + Sync> {
    config.ensure_blockchain_feature();

    #[cfg(feature = "ethereum")]
    if config.is_ethereum_mode() {
        return Box::pin(create_ethereum_keys_service(config, crypto_service)).await;
    }

    #[cfg(feature = "casper")]
    if config.is_casper_mode() {
        return Box::pin(create_casper_keys_service(config, crypto_service)).await;
    }

    #[cfg(feature = "cosmos")]
    if config.is_cosmos_mode() {
        return Box::pin(create_cosmos_keys_service(config, crypto_service)).await;
    }

    panic!(
        "Blockchain mode {:?} is not supported by this build. Enable the matching Cargo feature (casper, ethereum, or cosmos), or build with --features all.",
        config.get_blockchain_mode()
    );
}

#[cfg(feature = "ethereum")]
async fn create_ethereum_keys_service(
    config: &Config,
    crypto_service: CryptoService,
) -> Box<dyn KeysServiceTrait + Send + Sync> {
    if config.is_testing_mode() {
        Box::new(
            services::mocks::mock_ethereum_keys_service::MockEthereumKeysService::new(
                config.clone(),
                crypto_service,
            )
            .expect("Failed to initialize MockEthereumKeysService"),
        )
    } else {
        Box::new(
            services::ethereum_keys_service::EthereumKeysService::new(
                config.clone(),
                crypto_service,
            )
            .await
            .expect("Failed to initialize EthereumKeysService"),
        )
    }
}

#[cfg(feature = "casper")]
async fn create_casper_keys_service(
    config: &Config,
    crypto_service: CryptoService,
) -> Box<dyn KeysServiceTrait + Send + Sync> {
    if config.is_testing_mode() {
        Box::new(
            services::mocks::mock_casper_keys_service::MockCasperKeysService::new(
                config.clone(),
                crypto_service,
            )
            .expect("Failed to initialize MockCasperKeysService"),
        )
    } else {
        Box::new(
            services::casper_keys_service::CasperKeysService::new(config.clone(), crypto_service)
                .await
                .expect("Failed to initialize CasperKeysService"),
        )
    }
}

#[cfg(feature = "cosmos")]
async fn create_cosmos_keys_service(
    config: &Config,
    crypto_service: CryptoService,
) -> Box<dyn KeysServiceTrait + Send + Sync> {
    if config.is_testing_mode() {
        Box::new(
            services::mocks::mock_cosmos_keys_service::MockCosmosKeysService::new(
                config.clone(),
                crypto_service,
            )
            .expect("Failed to initialize MockCosmosKeysService"),
        )
    } else {
        Box::new(
            services::cosmos_keys_service::CosmosKeysService::new(config.clone(), crypto_service)
                .await
                .expect("Failed to initialize CosmosKeysService"),
        )
    }
}

/// Creates an Axum application instance with the given configuration.
///
/// # Panics
///
/// This function will panic if:
/// - Loading the WASM module fails.
/// - Initializing the `CryptoService` fails.
/// - Initializing the key service fails.
/// - `BLOCKCHAIN_MODE` requests a chain whose Cargo feature was not enabled at compile time.
///
/// These errors are propagated via calls to `expect` / `panic`.
pub async fn create_app(config: Config) -> Router {
    let wasm_loader = WasmLoader::new(WASM_PATH)
        .await
        .expect("Failed to load WASM module");

    let crypto_service =
        CryptoService::new(&wasm_loader).expect("Failed to initialize CryptoService");

    let keys_service = create_keys_service(&config, crypto_service).await;

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

    let swagger_ui = SwaggerUi::new("/docs/").url("/docs/openapi.json", ApiDoc::openapi());

    app.merge(swagger_ui)
}

/// Starts the HTTP server with the given configuration.
///
/// # Errors
/// This function returns an error if the TCP listener cannot be bound,
/// or if the server fails to start.
pub async fn run_server(
    config: Config,
) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    config.ensure_blockchain_feature();
    let app = create_app(config.clone()).await;

    let addr = format!("{}:{}", config.get_addr(), config.get_port());
    info!("Listening on {addr}");

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

    #[cfg(feature = "casper")]
    #[tokio::test]
    async fn test_run_server_creates_app() {
        let config = ConfigBuilder::new()
            .with_testing_mode(true)
            .with_port(0)
            .build();

        // Spawn the server in a background task but abort immediately,
        // just test that it starts without panics or errors.
        let server_future = task::spawn(async move { Box::pin(run_server(config)).await });

        // Wait briefly or abort since we don't want it running forever.
        // Here just wait a little and then abort.
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        server_future.abort();

        // If spawn didn't panic, assume success.
    }

    #[cfg(all(test, feature = "casper"))]
    mod tests_create_app {
        use super::*;
        use crate::config::ConfigBuilder;
        use axum::http::{self, Method, StatusCode};
        use tower::ServiceExt;

        async fn oneshot(
            app: &axum::Router,
            method: Method,
            uri: &str,
        ) -> axum::http::Response<axum::body::Body> {
            app.clone()
                .oneshot(
                    http::Request::builder()
                        .method(method)
                        .uri(uri)
                        .body(axum::body::Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap()
        }

        #[tokio::test]
        async fn test_create_app_routes() {
            let config = ConfigBuilder::new()
                .with_testing_mode(true)
                .with_delete_mode(true)
                .with_list_mode(true)
                .build();

            let app = create_app(config.clone()).await;

            assert_eq!(
                oneshot(&app, Method::GET, "/").await.status(),
                StatusCode::OK
            );

            assert!(
                oneshot(&app, Method::DELETE, "/deleteKey?key=test_key")
                    .await
                    .status()
                    .is_success(),
                "Expected /deleteKey route to exist or return 404",
            );

            assert_eq!(
                oneshot(&app, Method::GET, "/listKeys").await.status(),
                StatusCode::NOT_FOUND,
                "Expected /listKeys with no keys to return 404"
            );

            assert_ne!(
                oneshot(&app, Method::POST, "/createKey").await.status(),
                StatusCode::NOT_FOUND
            );

            assert!(
                oneshot(&app, Method::GET, "/listKeys")
                    .await
                    .status()
                    .is_success(),
                "Expected /listKeys route to exist"
            );

            assert_ne!(
                oneshot(&app, Method::POST, "/signTransactionHash")
                    .await
                    .status(),
                StatusCode::NOT_FOUND,
                "Expected /signTransactionHash route to exist"
            );

            assert_ne!(
                oneshot(&app, Method::POST, "/signTransaction")
                    .await
                    .status(),
                StatusCode::NOT_FOUND,
                "Expected /signTransaction route to exist"
            );

            assert_ne!(
                oneshot(&app, Method::GET, "/verifySignature")
                    .await
                    .status(),
                StatusCode::NOT_FOUND,
                "Expected /verifySignature route to exist"
            );

            assert!(
                oneshot(&app, Method::GET, "/docs/openapi.json")
                    .await
                    .status()
                    .is_success(),
                "Expected OpenAPI JSON /docs/openapi.json route to exist and respond successfully"
            );
        }
    }
}
