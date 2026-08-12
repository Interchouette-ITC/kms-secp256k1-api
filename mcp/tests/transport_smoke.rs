//! MCP transport smoke: Streamable HTTP initialize + tools/list, stdio initialize.

use serde_json::Value;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

fn free_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral");
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    port
}

fn mcp_bin() -> String {
    // Prefer the binary cargo just built for this test package (always matches CARGO_PKG_VERSION).
    if let Some(p) = option_env!("CARGO_BIN_EXE_kms-secp256k1-api-mcp") {
        return p.to_string();
    }
    // Fallback when running the test binary outside `cargo test`.
    let release = env!("CARGO_MANIFEST_DIR").to_string() + "/target/release/kms-secp256k1-api-mcp";
    let debug = env!("CARGO_MANIFEST_DIR").to_string() + "/target/debug/kms-secp256k1-api-mcp";
    if std::path::Path::new(&release).is_file() {
        release
    } else {
        debug
    }
}

fn repo_root() -> String {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("mcp parent")
        .to_string_lossy()
        .into_owned()
}

/// Parse JSON-RPC from plain JSON or Streamable HTTP SSE (`data: {...}`).
fn jsonrpc_from_body(raw: &str) -> Value {
    let trimmed = raw.trim();
    if trimmed.starts_with('{') {
        return serde_json::from_str(trimmed).expect("plain jsonrpc");
    }
    for line in trimmed.lines() {
        let line = line.trim();
        let Some(payload) = line.strip_prefix("data:") else {
            continue;
        };
        let payload = payload.trim();
        if payload.is_empty() || payload == "[DONE]" {
            continue;
        }
        if let Ok(v) = serde_json::from_str::<Value>(payload) {
            return v;
        }
    }
    panic!("no JSON-RPC in body: {raw}");
}

#[tokio::test]
async fn http_initialize_and_tools_list() {
    let bin = mcp_bin();
    assert!(
        std::path::Path::new(&bin).is_file(),
        "missing MCP binary at {bin}; run `cargo build --release` or `cargo test` first"
    );
    let port = free_port();
    let addr = format!("127.0.0.1:{port}");
    let mut child = Command::new(&bin)
        .args(["--http", "--listen", &addr])
        .env("KMS_API_ROOT", repo_root())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn mcp http");

    // Wait for listen
    let url = format!("http://{addr}/mcp");
    let client = reqwest::Client::new();
    let mut ready = false;
    for _ in 0..40 {
        thread::sleep(Duration::from_millis(100));
        let probe = client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": {},
                    "clientInfo": {"name": "transport-smoke", "version": "0.0.1"}
                }
            }))
            .send()
            .await;
        if let Ok(resp) = probe {
            let sid = resp
                .headers()
                .get("mcp-session-id")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("")
                .to_string();
            let raw = resp.text().await.expect("init body");
            let body = jsonrpc_from_body(&raw);
            assert_eq!(
                body["result"]["serverInfo"]["name"], "kms-secp256k1-api",
                "{body}"
            );
            assert_eq!(
                body["result"]["serverInfo"]["version"],
                env!("CARGO_PKG_VERSION")
            );

            let _ = client
                .post(&url)
                .header("Content-Type", "application/json")
                .header("Accept", "application/json, text/event-stream")
                .header("mcp-session-id", &sid)
                .json(&serde_json::json!({
                    "jsonrpc": "2.0",
                    "method": "notifications/initialized"
                }))
                .send()
                .await;

            let tools_resp = client
                .post(&url)
                .header("Content-Type", "application/json")
                .header("Accept", "application/json, text/event-stream")
                .header("mcp-session-id", &sid)
                .json(&serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": 2,
                    "method": "tools/list",
                    "params": {}
                }))
                .send()
                .await
                .expect("tools/list");
            let tools_raw = tools_resp.text().await.expect("tools body");
            let tools_body = jsonrpc_from_body(&tools_raw);
            let tools = tools_body["result"]["tools"]
                .as_array()
                .expect("tools array");
            let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
            assert!(names.contains(&"kms_create_key"), "{names:?}");
            assert!(names.contains(&"kms_docker_run_localstack"), "{names:?}");
            assert!(names.contains(&"kms_stack_start"), "{names:?}");
            assert!(
                names.len() >= 30,
                "expected full tool catalog, got {}",
                names.len()
            );
            ready = true;
            break;
        }
    }
    let _ = child.kill();
    let _ = child.wait();
    assert!(ready, "MCP HTTP did not become ready on {addr}");
}

#[test]
fn stdio_initialize() {
    let bin = mcp_bin();
    assert!(
        std::path::Path::new(&bin).is_file(),
        "missing MCP binary at {bin}"
    );
    let mut child = Command::new(&bin)
        .env("KMS_API_ROOT", repo_root())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn mcp stdio");

    {
        let stdin = child.stdin.as_mut().expect("stdin");
        writeln!(
            stdin,
            r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"protocolVersion":"2024-11-05","capabilities":{{}},"clientInfo":{{"name":"stdio-smoke","version":"0.0.1"}}}}}}"#
        )
        .expect("write initialize");
        let _ = stdin.flush();
    }

    let ver = env!("CARGO_PKG_VERSION");
    let stdout = child.stdout.take().expect("stdout");
    // Kill after 5s so Content-Length framing (no newlines) cannot block read_line forever in CI.
    let killer_pid = child.id();
    thread::spawn(move || {
        thread::sleep(Duration::from_secs(5));
        let _ = Command::new("kill").arg(killer_pid.to_string()).status();
    });
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    let mut buf = String::new();
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while std::time::Instant::now() < deadline {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {
                buf.push_str(&line);
                if buf.contains("kms-secp256k1-api") && buf.contains(ver) {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    let _ = child.kill();
    let _ = child.wait();
    assert!(
        buf.contains("kms-secp256k1-api"),
        "stdio initialize missing server name: {buf}"
    );
    assert!(
        buf.contains(ver),
        "stdio initialize missing version {ver}: {buf}"
    );
}
