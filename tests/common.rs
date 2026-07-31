//! Shared integration harness: bind `127.0.0.1:0`, serve [`create_app`], readiness poll,
//! return (`JoinHandle`, `base_url`).

use kms_secp256k1_api::{config::Config, create_app};
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;
use tokio::task::JoinHandle;

/// Starts an integration test server on an ephemeral port and waits until `/` is ready.
///
/// # Panics
///
/// Panics if the listener cannot bind, the server task fails, or readiness never succeeds.
#[must_use]
pub fn start_test_server(
    config: Config,
) -> Pin<Box<dyn Future<Output = (JoinHandle<()>, String)> + Send>> {
    Box::pin(async move {
        let app = create_app(config).await;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("Failed to bind test listener");
        let addr = listener
            .local_addr()
            .expect("Failed to read test listener address");
        let base_url = format!("http://{addr}");

        let handle = tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("Test server failed");
        });

        let client = reqwest::Client::new();
        for _ in 0..100 {
            if let Ok(resp) = client.get(format!("{base_url}/")).send().await
                && resp.status().is_success()
            {
                return (handle, base_url);
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        panic!("Test server at {base_url} did not become ready");
    })
}
