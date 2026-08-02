//! Host mock API: createKey → listKeys → deleteKey via MCP client helpers.

use kms_secp256k1_api_mcp::client;
use kms_secp256k1_api_mcp::ops;
use kms_secp256k1_api_mcp::paths::DEFAULT_API_URL;
use serde_json::Value;
use std::env;
use std::time::Duration;

async fn wait_hello(seconds: u64) -> bool {
    let deadline = std::time::Instant::now() + Duration::from_secs(seconds);
    while std::time::Instant::now() < deadline {
        let hello = client::hello(None).await;
        if hello.contains("HTTP 200") {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    false
}

#[tokio::test]
async fn mock_api_create_list_delete_roundtrip() {
    // Use a high port to avoid colliding with a developer API on :4000.
    let port: u16 = 4011;
    let base = format!("http://127.0.0.1:{port}");
    env::set_var("KMS_API_URL", &base);
    env::set_var(
        "KMS_API_ROOT",
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap(),
    );

    // casper-only: enough for mock create/list/delete; avoids cold-compiling ethereum/cosmos in CI.
    let start = ops::api_start(Some("casper"), Some(port));
    assert!(
        start.contains("started") || start.contains("already running"),
        "api_start failed: {start}"
    );
    if !wait_hello(60).await {
        let log_tail = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".run/api.log"),
        )
        .unwrap_or_else(|e| format!("(could not read api.log: {e})"));
        let tail: String = log_tail
            .chars()
            .rev()
            .take(4000)
            .collect::<String>()
            .chars()
            .rev()
            .collect();
        panic!("API did not become ready on {base}; last start:\n{start}\n--- api.log (tail) ---\n{tail}");
    }

    let created = client::create_key().await;
    assert!(
        created.contains("HTTP 201") || created.contains("public_key"),
        "create_key: {created}"
    );
    let json_line = created
        .lines()
        .find(|l| l.trim_start().starts_with('{'))
        .unwrap_or("");
    let body: Value = serde_json::from_str(json_line).unwrap_or(Value::Null);
    let key = body
        .get("address")
        .or_else(|| body.get("public_key"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    assert!(!key.is_empty(), "no address/public_key in {created}");

    let listed = client::list_keys().await;
    assert!(
        listed.contains("HTTP 200") || listed.contains(&key),
        "list_keys: {listed}"
    );

    let deleted = client::delete_key(&key).await;
    assert!(
        deleted.contains("HTTP 200") || deleted.contains("deleted"),
        "delete_key: {deleted}"
    );

    let stop = ops::api_stop();
    assert!(
        stop.contains("stopped") || stop.contains("nothing to stop"),
        "api_stop: {stop}"
    );

    env::set_var("KMS_API_URL", DEFAULT_API_URL);
}

/// LocalStack + API compose stack. Run with `KMS_MCP_LIVE=1` (needs Docker).
#[tokio::test]
#[ignore = "set KMS_MCP_LIVE=1 and Docker; long"]
async fn live_localstack_stack_create_delete() {
    if env::var("KMS_MCP_LIVE").ok().as_deref() != Some("1") {
        return;
    }
    env::set_var(
        "KMS_API_ROOT",
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap(),
    );
    env::set_var("KMS_API_URL", "http://127.0.0.1:4001");

    let start = ops::stack_start();
    assert!(
        start.contains("ok") || start.contains("healthy") || start.contains("Set KMS_API_URL"),
        "stack_start: {start}"
    );

    let mut ready = false;
    for _ in 0..60 {
        let hello = client::hello(None).await;
        if hello.contains("HTTP 200") {
            ready = true;
            break;
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    assert!(ready, "stack API not ready");

    let created = client::create_key().await;
    assert!(
        created.contains("HTTP 201") || created.contains("public_key"),
        "create_key: {created}"
    );
    let json_line = created
        .lines()
        .find(|l| l.trim_start().starts_with('{'))
        .unwrap_or("");
    let body: Value = serde_json::from_str(json_line).unwrap_or(Value::Null);
    let key = body
        .get("address")
        .or_else(|| body.get("public_key"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    assert!(!key.is_empty(), "no key in {created}");

    let deleted = client::delete_key(&key).await;
    assert!(
        deleted.contains("HTTP 200") || deleted.contains("deleted"),
        "delete_key: {deleted}"
    );

    let stop = ops::stack_stop();
    assert!(
        stop.contains("ok") || stop.contains("down"),
        "stack_stop: {stop}"
    );
}
