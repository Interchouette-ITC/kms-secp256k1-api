use kms_secp256k1_api::{config::ConfigBuilder, run_server};
use tokio::task;

async fn start_server(delete_mode: bool, ethereum_mode: bool) -> task::JoinHandle<()> {
    let mut builder = ConfigBuilder::new().with_delete_mode(delete_mode);

    if ethereum_mode {
        builder = builder.with_ethereum_mode();
    }

    let config = builder.build();

    task::spawn(async move {
        let _ = run_server(config).await;
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use kms_secp256k1_api::{constants::CASPER_PUBLIC_KEY_PREFIXED, routes::CreateKeyResponse};
    use serial_test::serial;
    use std::time::Duration;

    #[tokio::test]
    #[serial]
    async fn test_delete_key_returns_200_when_key_exists() {
        let delete_mode = true;
        let ethereum_mode = false;
        let server_handle = start_server(delete_mode, ethereum_mode).await;
        tokio::time::sleep(Duration::from_secs(1)).await;

        let client = reqwest::Client::new();

        // Create a key first
        let create_resp = client
            .post("http://127.0.0.1:4000/createKey")
            .send()
            .await
            .expect("Failed to send createKey request");

        assert!(create_resp.status().is_success());

        let created: CreateKeyResponse = create_resp
            .json()
            .await
            .expect("Failed to parse createKey response");

        // Delete the created key
        let url = format!("http://127.0.0.1:4000/deleteKey?key={}", created.address);
        let delete_resp = client
            .delete(&url)
            .send()
            .await
            .expect("Failed to send deleteKey request");

        assert!(delete_resp.status().is_success());

        let body = delete_resp.text().await.expect("Failed to read body");

        assert!(
            body.contains("\"deleted\":true"),
            "Expected successful deletion, got body: {body}"
        );

        server_handle.abort();
    }

    #[tokio::test]
    #[serial]
    async fn test_delete_key_returns_200_when_key_does_not_exist() {
        let delete_mode = true;
        let ethereum_mode = false;
        let server_handle = start_server(delete_mode, ethereum_mode).await;
        tokio::time::sleep(Duration::from_secs(1)).await;

        let client = reqwest::Client::new();

        let fake_key = CASPER_PUBLIC_KEY_PREFIXED;
        let url = format!("http://127.0.0.1:4000/deleteKey?key={fake_key}");
        let resp = client
            .delete(&url)
            .send()
            .await
            .expect("Failed to send deleteKey request");

        assert!(resp.status().is_success());

        let body = resp.text().await.expect("Failed to read body");

        assert!(
            body.contains("\"deleted\":false"),
            "Expected deleted:false for non-existent key, got: {body}"
        );

        server_handle.abort();
    }

    #[tokio::test]
    #[serial]
    async fn test_delete_key_returns_404_when_disabled() {
        let delete_mode = false;
        let ethereum_mode = false;
        let server_handle = start_server(delete_mode, ethereum_mode).await;
        tokio::time::sleep(Duration::from_secs(1)).await;

        let client = reqwest::Client::new();

        let url = "http://127.0.0.1:4000/deleteKey?key=dummykey";
        let resp = client
            .delete(url)
            .send()
            .await
            .expect("Failed to send deleteKey request");

        assert!(resp.status().is_client_error());

        server_handle.abort();
    }

    #[tokio::test]
    #[serial]
    async fn test_delete_key_returns_200_when_key_exists_from_address() {
        let delete_mode = true;
        let ethereum_mode = false;
        let server_handle = start_server(delete_mode, ethereum_mode).await;
        tokio::time::sleep(Duration::from_secs(1)).await;

        let client = reqwest::Client::new();

        // Create a key first
        let create_resp = client
            .post("http://127.0.0.1:4000/createKey")
            .send()
            .await
            .expect("Failed to send createKey request");

        assert!(create_resp.status().is_success());

        let created: CreateKeyResponse = create_resp
            .json()
            .await
            .expect("Failed to parse createKey response");

        // Delete the created key
        let url = format!("http://127.0.0.1:4000/deleteKey?key={}", created.address);
        let delete_resp = client
            .delete(&url)
            .send()
            .await
            .expect("Failed to send deleteKey request");

        assert!(delete_resp.status().is_success());

        let body = delete_resp.text().await.expect("Failed to read body");

        assert!(
            body.contains("\"deleted\":true"),
            "Expected successful deletion, got body: {body}"
        );

        server_handle.abort();
    }

    #[tokio::test]
    #[serial]
    async fn test_delete_key_returns_200_when_key_does_not_exist_from_address() {
        let delete_mode = true;
        let ethereum_mode = false;
        let server_handle = start_server(delete_mode, ethereum_mode).await;
        tokio::time::sleep(Duration::from_secs(1)).await;

        let client = reqwest::Client::new();

        let fake_key = "fake_address";
        let url = format!("http://127.0.0.1:4000/deleteKey?key={fake_key}");
        let resp = client
            .delete(&url)
            .send()
            .await
            .expect("Failed to send deleteKey request");

        assert!(resp.status().is_success());

        let body = resp.text().await.expect("Failed to read body");

        assert!(
            body.contains("\"deleted\":false"),
            "Expected deleted:false for non-existent key, got: {body}"
        );

        server_handle.abort();
    }

    #[tokio::test]
    #[serial]
    async fn test_delete_eth_key_returns_200_when_key_exists() {
        let delete_mode = true;
        let ethereum_mode = true;
        let server_handle = start_server(delete_mode, ethereum_mode).await;
        tokio::time::sleep(Duration::from_secs(1)).await;

        let client = reqwest::Client::new();

        // Create a key first
        let create_resp = client
            .post("http://127.0.0.1:4000/createKey")
            .send()
            .await
            .expect("Failed to send createKey request");

        assert!(create_resp.status().is_success());

        let created: CreateKeyResponse = create_resp
            .json()
            .await
            .expect("Failed to parse createKey response");

        // Delete the created key
        let url = format!("http://127.0.0.1:4000/deleteKey?key={}", created.public_key);
        let delete_resp = client
            .delete(&url)
            .send()
            .await
            .expect("Failed to send deleteKey request");

        assert!(delete_resp.status().is_success());

        let body = delete_resp.text().await.expect("Failed to read body");

        assert!(
            body.contains("\"deleted\":true"),
            "Expected successful deletion, got body: {body}"
        );

        server_handle.abort();
    }

    #[tokio::test]
    #[serial]
    async fn test_delete_eth_key_returns_200_when_key_exists_from_address() {
        let delete_mode = true;
        let ethereum_mode = true;
        let server_handle = start_server(delete_mode, ethereum_mode).await;
        tokio::time::sleep(Duration::from_secs(1)).await;

        let client = reqwest::Client::new();

        // Create a key first
        let create_resp = client
            .post("http://127.0.0.1:4000/createKey")
            .send()
            .await
            .expect("Failed to send createKey request");

        assert!(create_resp.status().is_success());

        let created: CreateKeyResponse = create_resp
            .json()
            .await
            .expect("Failed to parse createKey response");

        // Delete the created key
        let url = format!("http://127.0.0.1:4000/deleteKey?key={}", created.address);
        let delete_resp = client
            .delete(&url)
            .send()
            .await
            .expect("Failed to send deleteKey request");

        assert!(delete_resp.status().is_success());

        let body = delete_resp.text().await.expect("Failed to read body");

        assert!(
            body.contains("\"deleted\":true"),
            "Expected successful deletion, got body: {body}"
        );

        server_handle.abort();
    }
}
