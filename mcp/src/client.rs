//! Thin HTTP client for the running kms-secp256k1-api.

use reqwest::Client;
use serde_json::Value;

use crate::paths::api_base_url;

fn client() -> Client {
    Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .unwrap_or_else(|_| Client::new())
}

fn format_response(status: reqwest::StatusCode, body: &str) -> String {
    format!("HTTP {status}\n{body}")
}

async fn text_or_err(result: Result<reqwest::Response, reqwest::Error>) -> String {
    match result {
        Ok(resp) => {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_else(|e| e.to_string());
            format_response(status, &body)
        }
        Err(e) => format!("request failed: {e}"),
    }
}

pub async fn hello(message: Option<&str>) -> String {
    let mut url = format!("{}/", api_base_url().trim_end_matches('/'));
    if let Some(m) = message {
        if !m.is_empty() {
            url = format!(
                "{}?message={}",
                url.trim_end_matches('/'),
                urlencoding_lite(m)
            );
        }
    }
    text_or_err(client().get(&url).send().await).await
}

pub async fn create_key() -> String {
    let url = format!("{}/createKey", api_base_url().trim_end_matches('/'));
    text_or_err(client().post(&url).send().await).await
}

pub async fn sign_transaction_hash(keys: &str, hash_hex: &str) -> String {
    let url = format!(
        "{}/signTransactionHash?keys={}",
        api_base_url().trim_end_matches('/'),
        urlencoding_lite(keys)
    );
    text_or_err(
        client()
            .post(&url)
            .header("Content-Type", "text/plain")
            .body(hash_hex.to_string())
            .send()
            .await,
    )
    .await
}

pub async fn sign_transaction(keys: &str, tx_json: &str) -> String {
    let url = format!(
        "{}/signTransaction?keys={}",
        api_base_url().trim_end_matches('/'),
        urlencoding_lite(keys)
    );
    let body: Value = match serde_json::from_str(tx_json) {
        Ok(v) => v,
        Err(e) => return format!("invalid JSON body: {e}"),
    };
    text_or_err(client().post(&url).json(&body).send().await).await
}

pub async fn verify_signature(
    key: &str,
    transaction_hash: &str,
    signature: &str,
    via_kms: Option<bool>,
) -> String {
    let mut url = format!(
        "{}/verifySignature?key={}&transaction_hash={}&signature={}",
        api_base_url().trim_end_matches('/'),
        urlencoding_lite(key),
        urlencoding_lite(transaction_hash),
        urlencoding_lite(signature)
    );
    if let Some(v) = via_kms {
        url.push_str(&format!("&via_kms={v}"));
    }
    text_or_err(client().get(&url).send().await).await
}

pub async fn delete_key(key: &str) -> String {
    let url = format!(
        "{}/deleteKey?key={}",
        api_base_url().trim_end_matches('/'),
        urlencoding_lite(key)
    );
    text_or_err(client().delete(&url).send().await).await
}

pub async fn list_keys() -> String {
    let url = format!("{}/listKeys", api_base_url().trim_end_matches('/'));
    text_or_err(client().get(&url).send().await).await
}

pub async fn openapi() -> String {
    let url = format!("{}/docs/openapi.json", api_base_url().trim_end_matches('/'));
    match client().get(&url).send().await {
        Ok(resp) => {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_else(|e| e.to_string());
            if let Ok(v) = serde_json::from_str::<Value>(&body) {
                let paths = v
                    .get("paths")
                    .and_then(|p| p.as_object())
                    .map(|o| {
                        let mut keys: Vec<_> = o.keys().cloned().collect();
                        keys.sort();
                        keys.join("\n  ")
                    })
                    .unwrap_or_default();
                let title = v
                    .pointer("/info/title")
                    .and_then(|t| t.as_str())
                    .unwrap_or("(no title)");
                let version = v
                    .pointer("/info/version")
                    .and_then(|t| t.as_str())
                    .unwrap_or("?");
                return format!("HTTP {status}\n{title} v{version}\npaths:\n  {paths}");
            }
            format_response(status, &body)
        }
        Err(e) => format!("request failed: {e}"),
    }
}

/// Minimal query escaping without an extra crate.
fn urlencoding_lite(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(char::from(b));
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::test_env::ENV_LOCK;
    use std::env;

    #[test]
    fn urlencoding_preserves_hex() {
        assert_eq!(urlencoding_lite("0xabc"), "0xabc");
        assert_eq!(urlencoding_lite("a b"), "a+b");
    }

    #[test]
    fn api_base_url_env() {
        let _g = ENV_LOCK.lock().unwrap();
        env::set_var("KMS_API_URL", "http://127.0.0.1:4001");
        assert_eq!(api_base_url(), "http://127.0.0.1:4001");
        env::remove_var("KMS_API_URL");
    }
}
