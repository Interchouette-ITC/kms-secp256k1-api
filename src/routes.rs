#![allow(clippy::needless_for_each)]
use crate::config::Config;
use crate::{AppState, VERSION};
use axum::Json;
use axum::{Extension, response::IntoResponse};
use axum_extra::extract::Query;
use hyper::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashSet;
use utoipa::{IntoParams, OpenApi, ToSchema};

#[derive(OpenApi)]
#[openapi(paths(
    hello,
    create_keypair,
    sign_transaction_hash,
    sign_transaction,
    verify_signature,
    delete_key,
    list_keys,
))]
pub struct ApiDoc;

#[derive(Deserialize, IntoParams)]
pub struct HelloParams {
    message: Option<String>,
}

#[derive(Serialize, ToSchema)]
struct HelloResponse {
    message: String,
}

#[utoipa::path(
    get,
    path = "/",
    responses(
        (status = 200, description = "Hello message", body = HelloResponse)
    )
)]
pub async fn hello(
    Query(params): Query<HelloParams>,
    Extension(config): Extension<Config>,
) -> impl IntoResponse {
    let message = if config.is_testing_mode() {
        params.message.map_or_else(
            || "KMS TESTING_MODE".to_string(),
            |message| format!("KMS TESTING_MODE {message}"),
        )
    } else {
        params.message.unwrap_or_else(|| "KMS".to_string())
    };

    format!("Hello {message}! Version: {}", *VERSION)
}

#[derive(Serialize, ToSchema, Deserialize, Debug, Clone)]
pub struct CreateKeyResponse {
    pub address: String,
    pub public_key: String,
}

#[utoipa::path(
    post,
    path = "/createKey",
    responses(
        (status = 201, description = "Successful key generation", body = CreateKeyResponse),
    ),
    tag = "Key Management"
)]
pub async fn create_keypair(Extension(state): Extension<AppState>) -> impl IntoResponse {
    let mut keys_service = state.keys_service.lock().await;

    match keys_service.create_key(&state.config).await {
        Ok(key) => (
            StatusCode::CREATED,
            Json(json!({
                "public_key": key.public_key,
                "address": key.address
            })),
        ),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": err })),
        ),
    }
}

#[derive(Serialize, ToSchema, Deserialize, Debug)]
pub struct Approval {
    pub signer: String,
    pub r: String,
    pub s: String,
    pub v: String,
    pub hash: String,
    pub signature: String,
}

#[utoipa::path(
    post,
    path = "/signTransactionHash",
    params(
        ("keys" = Vec<String>, Query, description = "List of addresses or public keys to sign the transaction with"),
    ),
    request_body(
        content = String,
        description = "The transaction hash as hexadecimal",
        example = "0x..."
    ),
    responses(
        (status = 200, description = "Signatures for each key", body = Vec<Approval>),
        (status = 500, description = "Internal server error", body = String)
    ),
    tag = "Signature Management"
)]
pub async fn sign_transaction_hash(
    Extension(state): Extension<AppState>,
    Query(query): Query<SignTransactionParams>,
    transaction_hash: String,
) -> impl IntoResponse {
    let mut keys_service = state.keys_service.lock().await;

    let unique_keys: HashSet<_> = query.keys.iter().cloned().collect();

    if unique_keys.len() > 10 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "Too many public keys"
            })),
        );
    }

    let mut approvals = Vec::new();

    for key in unique_keys {
        match keys_service
            .sign_transaction_hash(&state.config, &transaction_hash, &key)
            .await
        {
            Ok(sig) => {
                let sig_clean = sig.strip_prefix("0x").unwrap_or(&sig);
                let mut sig_bytes = match hex::decode(sig_clean) {
                    Ok(bytes) if bytes.len() == 64 || bytes.len() == 65 => bytes,
                    _ => {
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(json!({ "error": "Invalid signature format" })),
                        );
                    }
                };

                // Strip 1 byte Casper prefix for r and s, should not have v
                if state.config.is_casper_mode() {
                    sig_bytes = sig_bytes[1..].to_vec();
                }

                let r = &sig_bytes[0..32];
                let s = &sig_bytes[32..64];
                let v = if sig_bytes.len() == 65 {
                    sig_bytes[64]
                } else {
                    0 // default v for Casper or signatures without recovery id
                };

                approvals.push(Approval {
                    signer: key,
                    v: format!("{v:02x}"),
                    r: hex::encode(r),
                    s: hex::encode(s),
                    hash: transaction_hash.clone(),
                    signature: sig.clone(),
                });
            }
            Err(err) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": format!("Error signing for {}: {}", key, err) })),
                );
            }
        }
    }

    let approvals_json = serde_json::to_value(&approvals).unwrap_or_default();

    (StatusCode::OK, Json(approvals_json))
}

#[derive(Deserialize, IntoParams)]
pub struct SignTransactionParams {
    #[param(min_items = 1, max_items = 10)]
    pub keys: Vec<String>,
}

#[utoipa::path(
    post,
    path = "/signTransaction",
    params(
        ("keys" = Vec<String>, Query, description = "List of addresses or public keys to sign the transaction with"),
    ),
    request_body(
        content = Value,
        description = "The transaction (json format)",
        example = json!({}),
    ),
    responses(
        (status = 200, description = "Signed transaction", body = Value),
        (status = 500, description = "Internal server error", body = String)
    ),
    tag = "Signature Management"
)]
pub async fn sign_transaction(
    Extension(state): Extension<AppState>,
    Query(query): Query<SignTransactionParams>,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    let mut keys_service = state.keys_service.lock().await;

    let unique_keys: HashSet<_> = query.keys.iter().cloned().collect();

    if unique_keys.len() > 10 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "Too many public keys"
            })),
        );
    }

    // Start with the original unsigned transaction
    let mut current_signed = body.to_string();

    for key in unique_keys {
        match keys_service
            .sign_transaction(&state.config, &current_signed, &key)
            .await
        {
            Ok(signed) => {
                // Use this newly signed transaction as base for the next
                current_signed = signed;
            }
            Err(err) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": format!("Failed to sign with key {}: {}", key, err)
                    })),
                );
            }
        }
    }

    // Deserialize once at the end to return clean JSON (not a quoted string)
    let final_value: Value = match serde_json::from_str(&current_signed) {
        Ok(v) => v,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Failed to parse final signed transaction"
                })),
            );
        }
    };

    (StatusCode::OK, Json(final_value))
}

#[derive(Deserialize, IntoParams)]
pub struct VerifySignatureParams {
    pub transaction_hash: String,
    pub key: String,
    pub signature: String,
    pub via_kms: Option<bool>,
}

#[utoipa::path(
    get,
    path = "/verifySignature",
    params(
        ("key" = String, Query, description = "The address or public key of the key to verify against"),
        ("transaction_hash" = String, Query, description = "The original transaction hash"),
        ("signature" = String, Query, description = "The signature to verify (with prefix)"),
        ("via_kms" = Option<bool>, Query, description = "Whether to verify the signature using AWS KMS (default: false)")
    ),
    responses(
        (status = 200, description = "Verification result", body = bool),
        (status = 500, description = "Internal server error", body = String)
    ),
    tag = "Signature Management"
)]
pub async fn verify_signature(
    Extension(state): Extension<AppState>,
    Query(params): Query<VerifySignatureParams>,
) -> impl IntoResponse {
    let mut keys_service = state.keys_service.lock().await;

    let via_kms = params.via_kms.unwrap_or(false);

    let result = if via_kms {
        keys_service
            .verify_via_kms(&params.transaction_hash, &params.signature, &params.key)
            .await
    } else {
        keys_service
            .verify(&params.transaction_hash, &params.signature, &params.key)
            .await
    };

    match result {
        Ok(valid) => (StatusCode::OK, Json(json!({ "valid": valid }))),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": err })),
        ),
    }
}

#[derive(Deserialize, IntoParams)]
pub struct DeleteKeyParams {
    pub key: String,
}

#[utoipa::path(
    delete,
    path = "/deleteKey",
    params(
        ("key" = String, Query, description = "Address or public key of the key to delete")
    ),
    responses(
        (status = 200, description = "Key deletion status", body = bool),
        (status = 404, description = "Delete feature is disabled"),
        (status = 500, description = "Internal server error", body = String)
    ),
    tag = "Key Management"
)]
pub async fn delete_key(
    Extension(state): Extension<AppState>,
    Query(params): Query<DeleteKeyParams>,
) -> impl IntoResponse {
    let mut keys_service = state.keys_service.lock().await;

    let key = params.key.trim();

    if key.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Key cannot be empty" })),
        );
    }

    match keys_service.delete_key(key).await {
        Ok(result) => (StatusCode::OK, Json(json!({ "deleted": result }))),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": err })),
        ),
    }
}

#[derive(Serialize, Deserialize, ToSchema, Eq, PartialEq, Debug, Clone)]
struct KeyEntryResponse {
    pub address: String,
    pub public_key_base64: String,
    pub public_key: String,
    pub key_id: String,
}

#[utoipa::path(
    get,
    path = "/listKeys",
    responses(
        (status = 200, description = "List of keys", body = Vec<KeyEntryResponse>),
        (status = 404, description = "Listing of keys feature is disabled"),
        (status = 500, description = "Internal server error", body = String)
    ),
    tag = "Key Management"
)]
pub async fn list_keys(Extension(state): Extension<AppState>) -> impl IntoResponse {
    let mut keys_service = state.keys_service.lock().await;

    match keys_service.list_keys().await {
        Ok(keys) => {
            let keys_entries: Vec<KeyEntryResponse> = keys
                .into_iter()
                .map(|key_entry| KeyEntryResponse {
                    address: key_entry.address.to_string(),
                    public_key_base64: key_entry.public_key_base64.to_string(),
                    public_key: key_entry
                        .public_key
                        .as_deref()
                        .unwrap_or_default()
                        .to_string(),
                    key_id: key_entry.key_id.to_string(),
                })
                .collect();

            if keys_entries.is_empty() {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": "No keys found" })),
                );
            }

            (StatusCode::OK, Json(json!({ "keys": keys_entries })))
        }
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": err })),
        ),
    }
}

#[cfg(test)]
mod tests_routes {
    use super::*;
    use crate::{
        AppState,
        config::{Config, ConfigBuilder},
        constants::{CASPER_PUBLIC_KEY_PREFIXED, SIGNATURE_PREFIXED, TRANSACTION_HASH},
        services::keys_service::{KeyEntry, KeysServiceTrait},
    };
    use async_trait::async_trait;
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD;
    use http_body_util::BodyExt;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    #[derive(Default, Debug)]
    struct MockKeysService {
        keys: Vec<KeyEntry>,
    }

    #[async_trait]
    impl KeysServiceTrait for MockKeysService {
        async fn create_key(
            &mut self,
            _config: &crate::config::Config,
        ) -> Result<KeyEntry, String> {
            let public_key = CASPER_PUBLIC_KEY_PREFIXED.to_string();
            Ok(KeyEntry {
                public_key: Some(public_key).into(),
                address: "address".to_string().into(),
                public_key_base64: STANDARD.encode("public_key_base64").into(),
                key_id: "key_id".to_string().into(),
            })
        }

        async fn list_keys(&mut self) -> Result<Vec<KeyEntry>, String> {
            Ok(self.keys.clone())
        }

        async fn sign_transaction(
            &mut self,
            _config: &crate::config::Config,
            tx: &str,
            public_key: &str,
        ) -> Result<String, String> {
            if public_key == "fail" {
                return Err("Forced signature failure".to_string());
            }
            let mut value: serde_json::Value = serde_json::from_str(tx)
                .map_err(|e| format!("Invalid JSON in transaction: {e}"))?;

            match &mut value {
                serde_json::Value::Object(map) => match map.get_mut("signed_by") {
                    Some(serde_json::Value::Array(arr)) => {
                        arr.push(serde_json::Value::String(public_key.to_string()));
                    }
                    Some(_) => {
                        let old = map.remove("signed_by").unwrap();
                        map.insert(
                            "signed_by".to_string(),
                            serde_json::Value::Array(vec![
                                old,
                                serde_json::Value::String(public_key.to_string()),
                            ]),
                        );
                    }
                    None => {
                        map.insert(
                            "signed_by".to_string(),
                            serde_json::Value::Array(vec![serde_json::Value::String(
                                public_key.to_string(),
                            )]),
                        );
                    }
                },
                _ => {
                    return Err("Transaction JSON must be an object".to_string());
                }
            }
            serde_json::to_string(&value).map_err(|e| e.to_string())
        }

        async fn sign_transaction_hash(
            &mut self,
            _config: &crate::config::Config,
            _transaction_hash: &str,
            _public_key: &str,
        ) -> Result<String, String> {
            Ok(SIGNATURE_PREFIXED.to_string())
        }

        async fn verify(
            &mut self,
            _transaction_hash_hex: &str,
            signature_hex: &str,
            public_key: &str,
        ) -> Result<bool, String> {
            Ok(signature_hex == "valid" && public_key == "pubkey")
        }

        async fn verify_via_kms(
            &mut self,
            _transaction_hash_hex: &str,
            signature_hex: &str,
            public_key: &str,
        ) -> Result<bool, String> {
            Ok(signature_hex == "kms-valid" && public_key == "pubkey")
        }

        async fn delete_key(&mut self, key: &str) -> Result<bool, String> {
            let before = self.keys.len();
            self.keys.retain(|key_entry| {
                key != key_entry.address.as_str() && key_entry.public_key.as_deref() != Some(key)
            });
            let after = self.keys.len();
            Ok(after < before)
        }
    }

    #[tokio::test]
    async fn test_hello_with_custom_message() {
        let params = HelloParams {
            message: Some("World".to_string()),
        };

        let config = ConfigBuilder::new().with_testing_mode(false).build();

        let response = hello(Query(params), Extension(config))
            .await
            .into_response();

        let status = response.status();
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        assert_eq!(status, StatusCode::OK);
        assert!(body_str.contains("Hello World!"));
        assert!(body_str.contains(&format!("Version: {}", *VERSION)));
    }

    #[tokio::test]
    async fn test_hello_with_mock_mode() {
        let params = HelloParams { message: None };

        let config = ConfigBuilder::new().with_testing_mode(true).build();

        let response = hello(Query(params), Extension(config))
            .await
            .into_response();

        let status = response.status();
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        assert_eq!(status, StatusCode::OK);
        assert!(body_str.contains("Hello KMS TESTING_MODE!"));
    }

    #[tokio::test]
    async fn test_create_keypair_success() {
        let mock_service = MockKeysService::default();
        let state = AppState {
            keys_service: Arc::new(Mutex::new(Box::new(mock_service))),
            config: Config::default(),
        };

        let response = create_keypair(Extension(state)).await.into_response();

        assert_eq!(response.status(), StatusCode::CREATED);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        assert!(
            body_str.contains(CASPER_PUBLIC_KEY_PREFIXED),
            "Expected mocked public key in response"
        );
    }

    #[tokio::test]
    async fn test_list_keys_success() {
        let mock_service = MockKeysService {
            keys: vec![
                KeyEntry {
                    address: "address_1".to_string().into(),
                    public_key_base64: STANDARD.encode("public_key_1_base64").into(),
                    public_key: Some("public_key_1".to_string()).into(),
                    key_id: "key_id_1".to_string().into(),
                },
                KeyEntry {
                    address: "address_2".to_string().into(),
                    public_key_base64: STANDARD.encode("public_key_2_base64").into(),
                    public_key: Some("public_key_2".to_string()).into(),
                    key_id: "key_id_2".to_string().into(),
                },
            ],
        };

        let state = AppState {
            keys_service: Arc::new(Mutex::new(Box::new(mock_service))),
            config: Config::default(),
        };

        let response = list_keys(Extension(state)).await.into_response();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        let json_val: serde_json::Value = serde_json::from_str(&body_str).unwrap();
        let keys = json_val.get("keys").unwrap().as_array().unwrap();

        assert_eq!(keys.len(), 2);
        assert_eq!(keys[0]["address"], "address_1");
        assert_eq!(
            keys[0]["public_key_base64"],
            STANDARD.encode("public_key_1_base64")
        );
        assert_eq!(keys[0]["public_key"], "public_key_1");
        assert_eq!(keys[0]["key_id"], "key_id_1");
        assert_eq!(keys[1]["address"], "address_2");
        assert_eq!(
            keys[1]["public_key_base64"],
            STANDARD.encode("public_key_2_base64")
        );
        assert_eq!(keys[1]["public_key"], "public_key_2");
        assert_eq!(keys[1]["key_id"], "key_id_2");
    }

    #[tokio::test]
    async fn test_delete_key_success_from_address() {
        let mock_service = MockKeysService {
            keys: vec![
                KeyEntry {
                    address: "address_1".to_string().into(),
                    public_key_base64: STANDARD.encode("public_key_1_base64").into(),
                    public_key: Some("public_key_1".to_string()).into(),
                    key_id: "key_id_1".to_string().into(),
                },
                KeyEntry {
                    address: "address_2".to_string().into(),
                    public_key_base64: STANDARD.encode("public_key_2_base64").into(),
                    public_key: Some("public_key_2".to_string()).into(),
                    key_id: "key_id_2".to_string().into(),
                },
            ],
        };

        let state = AppState {
            keys_service: Arc::new(Mutex::new(Box::new(mock_service))),
            config: Config::default(),
        };

        let params = DeleteKeyParams {
            key: "address_1".to_string(),
        };

        let response = delete_key(Extension(state), Query(params))
            .await
            .into_response();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body_json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(body_json["deleted"], true);
    }

    #[tokio::test]
    async fn test_delete_key_success() {
        let mock_service = MockKeysService {
            keys: vec![
                KeyEntry {
                    address: "address_1".to_string().into(),
                    public_key_base64: STANDARD.encode("public_key_1_base64").into(),
                    public_key: Some("public_key_1".to_string()).into(),
                    key_id: "key_id_1".to_string().into(),
                },
                KeyEntry {
                    address: "address_2".to_string().into(),
                    public_key_base64: STANDARD.encode("public_key_2_base64").into(),
                    public_key: Some("public_key_2".to_string()).into(),
                    key_id: "key_id_2".to_string().into(),
                },
            ],
        };

        let state = AppState {
            keys_service: Arc::new(Mutex::new(Box::new(mock_service))),
            config: Config::default(),
        };

        let params = DeleteKeyParams {
            key: "public_key_1".to_string(),
        };

        let response = delete_key(Extension(state), Query(params))
            .await
            .into_response();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body_json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(body_json["deleted"], true);
    }

    #[tokio::test]
    async fn test_delete_key_with_none_or_empty_should_fail() {
        let mock_service = MockKeysService { keys: vec![] };

        let state = AppState {
            keys_service: Arc::new(Mutex::new(Box::new(mock_service))),
            config: Config::default(),
        };

        let params = DeleteKeyParams {
            key: "".to_string(),
        };

        let response = delete_key(Extension(state.clone()), Query(params))
            .await
            .into_response();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body_json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert!(
            body_json["error"]
                .as_str()
                .unwrap()
                .contains("Key cannot be empty")
        );
    }

    #[tokio::test]
    async fn test_delete_key_not_found() {
        let mock_service = MockKeysService {
            keys: vec![KeyEntry {
                address: "address_1".to_string().into(),
                public_key_base64: STANDARD.encode("public_key_1_base64").into(),
                public_key: Some("public_key_1".to_string()).into(),
                key_id: "key_id_1".to_string().into(),
            }],
        };

        let state = AppState {
            keys_service: Arc::new(Mutex::new(Box::new(mock_service))),
            config: Config::default(),
        };

        let params = DeleteKeyParams {
            key: "nonexistent".to_string(),
        };

        let response = delete_key(Extension(state), Query(params))
            .await
            .into_response();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body_json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(body_json["deleted"], false);
    }

    #[tokio::test]
    async fn test_verify_signature_basic_success() {
        let mock_service = MockKeysService::default();

        let state = AppState {
            keys_service: Arc::new(Mutex::new(Box::new(mock_service))),
            config: Config::default(),
        };

        let params = VerifySignatureParams {
            transaction_hash: "abc123".into(),
            signature: "valid".into(),
            key: "pubkey".into(),
            via_kms: Some(false),
        };

        let response = verify_signature(Extension(state), Query(params))
            .await
            .into_response();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["valid"], true);
    }

    #[tokio::test]
    async fn test_verify_signature_via_kms_success() {
        let mock_service = MockKeysService::default();

        let state = AppState {
            keys_service: Arc::new(Mutex::new(Box::new(mock_service))),
            config: Config::default(),
        };

        let params = VerifySignatureParams {
            transaction_hash: "abc123".into(),
            signature: "kms-valid".into(),
            key: "pubkey".into(),
            via_kms: Some(true),
        };

        let response = verify_signature(Extension(state), Query(params))
            .await
            .into_response();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["valid"], true);
    }

    #[tokio::test]
    async fn test_verify_signature_failure() {
        let mock_service = MockKeysService::default();

        let state = AppState {
            keys_service: Arc::new(Mutex::new(Box::new(mock_service))),
            config: Config::default(),
        };

        let params = VerifySignatureParams {
            transaction_hash: "abc123".into(),
            signature: "invalid".into(),
            key: "pubkey".into(),
            via_kms: Some(false),
        };

        let response = verify_signature(Extension(state), Query(params))
            .await
            .into_response();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["valid"], false);
    }

    #[tokio::test]
    async fn test_sign_transaction_hash_success() {
        let mock_service = MockKeysService::default();

        let state = AppState {
            keys_service: Arc::new(Mutex::new(Box::new(mock_service))),
            config: Config::default(),
        };

        let query = SignTransactionParams {
            keys: vec!["key1".to_string(), "key2".to_string(), "key1".to_string()],
        };

        let transaction_hash =
            "d2ab5d10a332cdf3222b7ffecb5abd07b44f338be7193775465e10b3e4fe0299".to_string();

        let response =
            sign_transaction_hash(Extension(state), Query(query), transaction_hash.clone())
                .await
                .into_response();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();

        let approvals: Vec<Approval> = serde_json::from_slice(&body).unwrap();
        assert_eq!(approvals.len(), 2);

        for approval in approvals {
            assert!(approval.signer == "key1" || approval.signer == "key2");
            assert_eq!(approval.signature, SIGNATURE_PREFIXED);
        }
    }

    #[tokio::test]
    async fn test_sign_transaction_hash_too_many_keys() {
        let mock_service = MockKeysService::default();

        let state = AppState {
            keys_service: Arc::new(Mutex::new(Box::new(mock_service))),
            config: Config::default(),
        };

        let mut keys = Vec::new();
        for i in 0..11 {
            keys.push(format!("key{i}"));
        }
        let query = SignTransactionParams { keys };

        let transaction_hash = TRANSACTION_HASH.to_string();

        let response = sign_transaction_hash(Extension(state), Query(query), transaction_hash)
            .await
            .into_response();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json_val: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(json_val["error"], "Too many public keys");
    }

    #[tokio::test]
    async fn test_sign_transaction_success() {
        let mock_service = MockKeysService::default();

        let state = AppState {
            keys_service: Arc::new(Mutex::new(Box::new(mock_service))),
            config: Config::default(),
        };

        let query = SignTransactionParams {
            keys: vec!["keyA".to_string(), "keyB".to_string()],
        };

        let transaction_json = json!({
            "foo": "bar"
        });

        let response = sign_transaction(Extension(state), Query(query), Json(transaction_json))
            .await
            .into_response();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();

        let signed_tx: serde_json::Value = serde_json::from_slice(&body).unwrap();

        let signed_by = signed_tx.get("signed_by").unwrap().as_array().unwrap();

        assert!(signed_by.iter().any(|v| v == "keyA"));
        assert!(signed_by.iter().any(|v| v == "keyB"));
    }

    #[tokio::test]
    async fn test_sign_transaction_too_many_keys() {
        let mock_service = MockKeysService::default();

        let state = AppState {
            keys_service: Arc::new(Mutex::new(Box::new(mock_service))),
            config: Config::default(),
        };

        let query = SignTransactionParams {
            keys: (0..11).map(|i| format!("key{i}")).collect(),
        };

        let transaction_json = json!({"foo": "bar"});

        let response = sign_transaction(Extension(state), Query(query), Json(transaction_json))
            .await
            .into_response();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(json["error"], "Too many public keys");
    }

    #[tokio::test]
    async fn test_sign_transaction_invalid_json() {
        let mock_service = MockKeysService::default();

        let state = AppState {
            keys_service: Arc::new(Mutex::new(Box::new(mock_service))),
            config: Config::default(),
        };

        let query = SignTransactionParams {
            keys: vec!["keyA".to_string()],
        };

        let transaction_json = json!(["not", "an", "object"]);

        let response = sign_transaction(Extension(state), Query(query), Json(transaction_json))
            .await
            .into_response();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(
            json["error"],
            "Failed to sign with key keyA: Transaction JSON must be an object"
        );
    }

    #[tokio::test]
    async fn test_sign_transaction_signature_failure() {
        let mock_service = MockKeysService::default();

        let state = AppState {
            keys_service: Arc::new(Mutex::new(Box::new(mock_service))),
            config: Config::default(),
        };

        let query = SignTransactionParams {
            keys: vec!["keyA".to_string(), "fail".to_string()],
        };

        let transaction_json = json!({"foo": "bar"});

        let response = sign_transaction(Extension(state), Query(query), Json(transaction_json))
            .await
            .into_response();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert!(
            json["error"]
                .as_str()
                .unwrap()
                .contains("Failed to sign with key fail")
        );
    }
}
