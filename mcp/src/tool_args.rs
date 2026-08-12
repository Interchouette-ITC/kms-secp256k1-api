//! JSON-schema parameter structs for rmcp `Parameters<T>` tool handlers.

use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Default, Deserialize, JsonSchema)]
pub struct FeaturesArgs {
    #[serde(default)]
    pub features: Option<String>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
pub struct ApiStartArgs {
    #[serde(default)]
    pub features: Option<String>,
    #[serde(default)]
    pub port: Option<u32>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
pub struct HelloArgs {
    #[serde(default)]
    pub message: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SignTransactionHashArgs {
    pub keys: String,
    pub hash_hex: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SignTransactionArgs {
    pub keys: String,
    pub tx_json: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VerifySignatureArgs {
    pub key: String,
    pub transaction_hash: String,
    pub signature: String,
    #[serde(default)]
    pub via_kms: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DeleteKeyArgs {
    pub key: String,
}
