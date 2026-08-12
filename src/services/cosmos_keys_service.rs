use crate::config::Config;
use crate::constants::{COSMOS_SECP_LEN, DEFAULT_COSMOS_HRP};
use crate::services::crypto_service::CryptoService;
use crate::services::keys_service::{KeyEntry, KeysService, KeysServiceTrait, SigEntry};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use cosmrs::{
    Any,
    crypto::PublicKey as CosmosSecp256k1PublicKey,
    proto::cosmos::tx::v1beta1::TxRaw,
    tendermint::{block, chain::Id},
    tx::{AuthInfo, Body, Fee, MessageExt, SignDoc, SignerInfo},
};
use k256::ecdsa::VerifyingKey;
use k256::{
    PublicKey,
    ecdsa::Signature,
    sha2::{Digest, Sha256},
};
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use std::str::FromStr;
use tracing::error;

pub struct CosmosKeysService {
    keys_service: KeysService,
    hrp: String,
}

#[async_trait::async_trait]
impl KeysServiceTrait for CosmosKeysService {
    /// Creates a new KMS key, derives its public key,
    /// registers an alias for it, and returns the formatted public key.
    ///
    /// # Arguments
    ///
    /// * `config` - Reference to the config struct used to decide prefixing.
    ///
    /// # Errors
    ///
    /// Returns an error if key creation, public key conversion, or alias creation fails.
    async fn create_key(&mut self, _config: &Config) -> crate::Result<KeyEntry> {
        let (key_id, public_key_base64, public_key) = self.keys_service.create_kms_key().await?;

        let key = self.resolve_key(&public_key)?;

        self.keys_service.create_alias(&key_id, &key).await?;

        Ok(KeyEntry {
            public_key: Some(public_key.clone()).into(),
            address: key.clone().into(),
            public_key_base64: public_key_base64.into(),
            key_id: key_id.into(),
        })
    }

    /// Signs a transaction hash using the provided public key and configuration mode.
    ///
    /// Delegates signing to the internal `sign` method.
    ///
    /// # Arguments
    ///
    /// * `config` - Reference to the application configuration to determine signing mode.
    /// * `transaction_hash` - The hash of the transaction to be signed.
    /// * `key` - The key alias corresponding to the private key for signing.
    ///
    /// # Errors
    ///
    /// Returns an error if the signing operation fails.
    async fn sign_transaction_hash(
        &mut self,
        config: &Config,
        transaction_hash: &str,
        key: &str,
    ) -> crate::Result<SigEntry> {
        Self::ensure_cosmos_mode(config)?;

        Self::validate_transaction_hash(transaction_hash)?;

        let key = self.resolve_key(key)?;
        let public_key = self.resolve_public_key(&key).await?;

        Self::validate_public_key(&public_key, "sign_transaction_hash")?;

        // Perform signing
        let signature = self.sign(transaction_hash, &key, &public_key).await?;

        Ok(SigEntry {
            address: key.into(),
            public_key: public_key.into(),
            signature: signature.into(),
        })
    }

    /// Signs a serialized Cosmos transaction using the provided public key.
    ///
    /// Parses the input transaction string, verifies the transaction hash and public key,
    /// signs the hash, attaches the signature to the transaction, and returns the signed
    /// transaction as a JSON string.
    ///
    /// # Arguments
    ///
    /// * `config` - Reference to the application configuration. Only Cosmos mode is supported.
    /// * `transaction_str` - A JSON string representing the transaction to be signed.
    /// * `key` - The key alias corresponding to the signing key.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The application is not in Cosmos mode.
    /// - The transaction JSON is invalid.
    /// - The transaction hash or public key is invalid.
    /// - The signing process fails.
    /// - The final transaction serialization fails.
    async fn sign_transaction(
        &mut self,
        config: &Config,
        transaction_str: &str,
        key: &str,
    ) -> crate::Result<String> {
        Self::ensure_cosmos_mode(config)?;
        let tx_json: serde_json::Value = serde_json::from_str(transaction_str)
            .map_err(|e| crate::KmsError::Msg(format!("Failed to parse JSON: {e}")))?;

        // Extract chain_id and address
        let chain_id = Id::from_str(&config.get_cosmos_chain_id())
            .map_err(|e| crate::KmsError::Msg(format!("Failed to fetch chain_id: {e}")))?;

        let key = self.resolve_key(key)?;
        let public_key = self.resolve_public_key(&key).await?;

        // Decode and validate public key bytes
        Self::validate_public_key(&public_key, "sign_transaction")?;

        // Deserialize TxBody
        let helper: BodyHelper = serde_json::from_value(tx_json["body"].clone())
            .map_err(|e| crate::KmsError::Msg(format!("Invalid TxBody: {e}")))?;

        let tx_body = helper.into_body()?;

        // Deserialize Fee
        let fee: Fee = serde_json::from_value(tx_json["auth_info"]["fee"].clone())
            .map_err(|e| crate::KmsError::Msg(format!("Invalid Fee: {e}")))?;

        let public_key_bytes = hex::decode(&public_key)
            .map_err(|e| crate::KmsError::Msg(format!("Invalid hex public key: {e}")))?;

        if public_key_bytes.len() != 33 {
            return Err(crate::KmsError::Msg(format!(
                "Invalid public key length: expected 33 bytes, got {}",
                public_key_bytes.len()
            )));
        }

        let pubkey_base64 = STANDARD.encode(public_key_bytes.clone());

        // Fetch account_number and sequence
        let mut account = fetch_account_info(&key, &pubkey_base64, config)
            .await
            .map_err(|e| crate::KmsError::Msg(format!("Failed to fetch account info: {e}")))?;

        let Some(fetched_pub_key) = account.pub_key.take() else {
            return Err(crate::KmsError::Msg(
                "Account info is missing pub_key".into(),
            ));
        };

        if fetched_pub_key.key != pubkey_base64 {
            return Err(crate::KmsError::Msg(format!(
                "Invalid fetched public key: got {}",
                fetched_pub_key.key
            )));
        }
        let auth_info = build_auth_info(&public_key_bytes, account.sequence, &fee)?;

        // Build SignDoc
        let sign_doc = SignDoc::new(&tx_body, &auth_info, &chain_id, account.sequence)
            .map_err(|e| crate::KmsError::Msg(format!("SignDoc error: {e}")))?;

        let sign_doc_bytes = sign_doc
            .into_bytes()
            .map_err(|e| crate::KmsError::Msg(format!("SignDoc encode error: {e}")))?;

        // Hash and sign
        let transaction_hash_bytes = Sha256::digest(&sign_doc_bytes);
        let transaction_hash = hex::encode(transaction_hash_bytes);

        Self::validate_transaction_hash(&transaction_hash)?;

        // Perform signing
        let signature_hex = self.sign(&transaction_hash, &key, &public_key).await?;

        let body_bytes = tx_body
            .into_bytes()
            .map_err(|e| crate::KmsError::Msg(format!("Failed to encode body: {e}")))?;
        let auth_info_bytes = auth_info
            .into_bytes()
            .map_err(|e| crate::KmsError::Msg(format!("Failed to encode auth_info: {e}")))?;

        let sig_bytes =
            hex::decode(signature_hex).map_err(|e| crate::KmsError::InvalidHex(e.to_string()))?;

        let signature = Signature::from_slice(&sig_bytes)
            .map_err(|e| crate::KmsError::Msg(format!("Invalid compact signature: {e}")))?;

        let tx_raw = TxRaw {
            body_bytes,
            auth_info_bytes,
            signatures: vec![signature.to_bytes().to_vec()],
        };

        let tx_raw_bytes = tx_raw
            .to_bytes()
            .map_err(|e| crate::KmsError::Msg(format!("Failed to encode TxRaw: {e}")))?;

        let base64_tx = STANDARD.encode(tx_raw_bytes);

        let broadcast_request = json!({
            "tx_bytes": base64_tx,
            "mode": "BROADCAST_MODE_SYNC"  // or "BLOCK" or "ASYNC"
        });

        let fee_amount_json = fee_amount_json(&fee);

        // Existing signatures from the original transaction (if any)
        let new_signature = signature_to_json(&key, &public_key, &signature, &transaction_hash);
        let signatures_array = append_signature_to_transaction(&tx_json, new_signature);

        // Prepare final JSON response
        let result = json!({
            "chain_id": chain_id,
            "body": tx_json["body"],
            "auth_info": {
                "signer_infos": [
                    {
                        "public_key": {
                            "@type": fetched_pub_key.key_type,
                            "key": fetched_pub_key.key,
                        },
                        "mode_info": {
                            "single": { "mode": "SIGN_MODE_DIRECT" }
                        },
                        "sequence": account.sequence,
                        "account_number": account.account_number
                    }
                ],
                "fee": {
                    "amount": fee_amount_json,
                    "gas_limit": fee.gas_limit
                }
            },
            "broadcast_request": broadcast_request,
            "signatures": signatures_array
        });

        // Return the wrapped transaction + signatures JSON
        serde_json::to_string(&result).map_err(|e| {
            crate::KmsError::Msg(format!("Failed to serialize final signed transaction: {e}"))
        })
    }

    async fn verify(
        &mut self,
        transaction_hash_hex: &str,
        signature_hex: &str,
        key: &str,
    ) -> crate::Result<bool> {
        let public_key = self.resolve_public_key(key).await?;
        self.keys_service
            .verify(transaction_hash_hex, signature_hex, &public_key)
    }

    async fn verify_via_kms(
        &mut self,
        transaction_hash_hex: &str,
        signature_hex: &str,
        key: &str,
    ) -> crate::Result<bool> {
        let address = self.resolve_key(key)?;
        let public_key = self.resolve_public_key(&address).await?;

        let is_verified =
            self.keys_service
                .verify(transaction_hash_hex, signature_hex, &public_key)?;
        if !is_verified {
            return Ok(false);
        }

        let signature = self.keys_service.crypto_service.unconvert(signature_hex)?;
        self.keys_service
            .kms_client_service
            .verify(transaction_hash_hex, &signature, &address)
            .await
            .map_err(|e| crate::KmsError::Msg(format!("Failed to verify signature with KMS: {e}")))
    }

    async fn delete_key(&mut self, key: &str) -> crate::Result<bool> {
        // Delegates to KeysService after resolving the Cosmos address.
        let key = self.resolve_key(key)?;
        self.keys_service.delete_key(&key).await
    }

    async fn list_keys(&mut self) -> crate::Result<Vec<KeyEntry>> {
        // Delegates to KeysService.
        self.keys_service.list_keys().await
    }
}

impl CosmosKeysService {
    /// Creates a new `CosmosKeysService` instance.
    ///
    /// # Arguments
    ///
    /// * `config` - The configuration object used to initialize the service.
    /// * `crypto_service` - The cryptographic service used for signing and key management.
    ///
    /// # Errors
    ///
    /// Returns an error string if the `KeysService` fails to initialize.
    ///
    pub async fn new(config: Config, crypto_service: CryptoService) -> crate::Result<Self> {
        let keys_service = KeysService::new(config.clone(), crypto_service).await?;
        let cosmos_hrp = config.get_cosmos_hrp();
        let hrp = match cosmos_hrp.as_str() {
            "" => DEFAULT_COSMOS_HRP,
            hrp => hrp,
        }
        .to_string();
        Ok(Self { keys_service, hrp })
    }

    async fn sign(
        &mut self,
        transaction_hash: &str,
        key: &str,
        public_key: &str,
    ) -> crate::Result<String> {
        let signature_hex = self
            .keys_service
            .sign(transaction_hash, key, None)
            .await
            .map_err(|e| crate::KmsError::SigningFailed(e.to_string()))?;

        Self::validate_signature_length(&signature_hex)?;

        let is_valid = self
            .verify(transaction_hash, &signature_hex, public_key)
            .await?;
        if !is_valid {
            return Err(crate::KmsError::PostSignVerifyFailed);
        }
        Ok(signature_hex)
    }

    /// Resolves the given key to a Cosmos address if it is a compressed public key.
    ///
    /// If the input is a valid 33-byte (compressed) secp256k1 public key, it is
    /// converted to a bech32-encoded Cosmos address using the configured HRP.
    /// Otherwise, the input is assumed to already be an address and returned as-is.
    ///
    /// # Arguments
    ///
    /// * `key` - A compressed secp256k1 public key (as a hex string) or a Cosmos address.
    ///
    /// # Errors
    ///
    /// Returns an error if the key is detected to be a public key and address conversion fails.
    ///
    fn resolve_key(&mut self, key: &str) -> crate::Result<String> {
        if key.len() == COSMOS_SECP_LEN {
            self.keys_service
                .crypto_service
                .address_cosmos(key, &self.hrp)
                .inspect_err(|e| {
                    error!(error = %e, "Failed to convert public key to address");
                })
        } else {
            Ok(key.to_string())
        }
    }

    /// Resolves a Cosmos-compatible public key from a key identifier.
    ///
    /// If the provided `key` is already a valid 33-byte compressed public key in hex format
    /// (expected length `COSMOS_SECP_LEN`), it is returned as-is. Otherwise, the key is
    /// assumed to be an alias or identifier managed by the KMS, and the corresponding
    /// public key is fetched from the KMS and serialized for use with Cosmos.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The KMS fails to return a public key for the given alias.
    /// - The returned key cannot be converted into a valid Cosmos-compatible public key.
    pub async fn resolve_public_key(&mut self, key: &str) -> crate::Result<String> {
        if key.len() == COSMOS_SECP_LEN {
            Ok(key.to_string())
        } else {
            let public_key = self
                .keys_service
                .kms_client_service
                .get_public_key(key)
                .await
                .map_err(|e| {
                    let msg = format!("Failed to get public key from alias with KMS: {e}");
                    error!("{}", msg);
                    crate::KmsError::Msg(msg)
                })?;

            self.keys_service
                .crypto_service
                .public_key(&public_key)
                .inspect_err(|e| {
                    error!(error = %e, "public_key conversion failed");
                })
        }
    }

    fn validate_public_key(public_key: &str, context: &str) -> crate::Result<()> {
        let public_key_bytes = hex::decode(public_key).map_err(|e| {
            let msg = format!("Error decoding public key in {context}: {e:?}");
            error!("{}", msg);
            crate::KmsError::Msg(msg)
        })?;

        PublicKey::from_sec1_bytes(&public_key_bytes).map_err(|e| {
            let msg = format!("Invalid public key bytes in {context}: {e:?}");
            error!("{}", msg);
            crate::KmsError::Msg(msg)
        })?;

        Ok(())
    }

    fn ensure_cosmos_mode(config: &Config) -> crate::Result<()> {
        if config.is_cosmos_mode() {
            Ok(())
        } else {
            Err(crate::KmsError::ModeMismatch { mode: "Cosmos" })
        }
    }

    fn validate_signature_length(hex: &str) -> crate::Result<()> {
        let bytes = hex::decode(hex).map_err(|_| crate::KmsError::InvalidSignatureHex)?;
        if bytes.len() == 64 {
            Ok(())
        } else {
            Err(crate::KmsError::InvalidSignatureLength { expected: 64 })
        }
    }

    fn validate_transaction_hash(transaction_hash: &str) -> crate::Result<()> {
        let hash_bytes = hex::decode(transaction_hash)
            .map_err(|e| crate::KmsError::InvalidTxHashHex(e.to_string()))?;

        if hash_bytes.len() != 32 {
            return Err(crate::KmsError::TxHashWrongLength);
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct PubKey {
    #[serde(rename = "@type")]
    pub key_type: String,
    pub key: String,
}

#[derive(Debug, Deserialize)]
pub struct BaseAccount {
    #[serde(rename = "@type")]
    pub account_type: String,
    pub address: String,
    pub pub_key: Option<PubKey>,
    pub account_number: u64,
    pub sequence: u64,
}

#[derive(Debug, Deserialize)]
struct AccountWrapper {
    pub account: BaseAccount,
}

#[derive(Deserialize)]
struct RawMsg {
    #[serde(rename = "@type")]
    pub type_url: String,
    #[serde(flatten)]
    pub value: serde_json::Value,
}

#[derive(Deserialize)]
pub struct BodyHelper {
    messages: Vec<RawMsg>,
    memo: Option<String>,
    timeout_height: Option<String>,
}

impl BodyHelper {
    /// Converts the transaction request into a `Body` used for signing and broadcasting.
    ///
    /// This method transforms JSON-encoded Cosmos messages into protobuf `Any` messages,
    /// parses the optional timeout height, and constructs the full `Body` structure for
    /// the transaction.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The `timeout_height` is not a valid integer.
    /// - Any message in the `messages` field fails to convert to a protobuf `Any`.
    pub fn into_body(self) -> crate::Result<Body> {
        let height = self
            .timeout_height
            .as_deref()
            .unwrap_or("0")
            .parse::<u64>()
            .map_err(|e| crate::KmsError::Msg(format!("Invalid timeout_height: {e}")))?;

        let any_msgs: crate::Result<Vec<Any>> =
            self.messages.iter().map(Self::json_msg_to_any).collect();
        let any_msgs = any_msgs?;

        let height_u32 = u32::try_from(height)
            .map_err(|_| crate::KmsError::Msg(format!("timeout_height too large: {height}")))?;

        Ok(Body::new(
            any_msgs,
            self.memo.unwrap_or_default(),
            block::Height::from(height_u32),
        ))
    }

    fn json_msg_to_any(raw: &RawMsg) -> crate::Result<Any> {
        let json_bytes = serde_json::to_vec(&raw.value)
            .map_err(|e| crate::KmsError::Msg(format!("JSON encode error: {e}")))?;

        Ok(Any {
            type_url: raw.type_url.clone(),
            value: json_bytes,
        })
    }
}

/// Fetches Cosmos `BaseAccount` information from the REST endpoint.
///
/// This function attempts to retrieve account metadata (e.g., sequence and account number)
/// from the configured Cosmos REST endpoint using the provided Bech32 address.
///
/// If the account does not exist on-chain (404 response), a default `BaseAccount` is returned
/// using the provided public key in base64 format.
///
/// # Arguments
///
/// * `address` - The Bech32 Cosmos address to fetch account info for.
/// * `pubkey_base64` - The base64-encoded public key used in case the account is not yet on-chain.
/// * `config` - A reference to the configuration containing the Cosmos REST URL.
///
/// # Returns
///
/// A `Result` containing either:
/// - `BaseAccount` with account metadata (either fetched or default), or
/// - An error if the request fails or the response cannot be parsed.
///
/// # Errors
///
/// Returns an error if the HTTP request fails, if the REST endpoint returns an unexpected status,
/// or if the response body cannot be parsed into an `AccountWrapper`.
pub async fn fetch_account_info(
    address: &str,
    pubkey_base64: &str,
    config: &Config,
) -> Result<BaseAccount, Box<dyn std::error::Error>> {
    let url = format!("{}{address}", config.get_cosmos_rest_url());
    let client = Client::new();

    let response = client.get(&url).send().await;

    if let Ok(resp) = response
        && resp.status().is_success()
    {
        let parsed: AccountWrapper = resp.json().await?;
        return Ok(parsed.account);
    }

    // On any failure (non-2xx status, error, etc.), return a default BaseAccount
    Ok(BaseAccount {
        account_type: "cosmos.auth.v1beta1.BaseAccount".to_string(),
        address: address.to_string(),
        pub_key: Some(PubKey {
            key_type: "/cosmos.crypto.secp256k1.PubKey".to_string(),
            key: pubkey_base64.to_string(),
        }),
        account_number: 0,
        sequence: 0,
    })
}

#[must_use]
pub fn signature_to_json(
    address: &str,
    public_key: &str,
    signature: &Signature,
    transaction_hash: &str,
) -> serde_json::Value {
    json!({
        "address": address,
        "signer": public_key,
        "v": "00", // Cosmos doesn't use EIP-155 v
        "r": format!("{:x}", signature.r()),
        "s": format!("{:x}", signature.s()),
        "hash": transaction_hash,
        "signature": hex::encode(signature.to_bytes()),
    })
}

#[must_use]
pub fn fee_amount_json(fee: &Fee) -> Vec<serde_json::Value> {
    fee.amount
        .iter()
        .map(|coin| {
            json!({
                "denom": coin.denom,
                "amount": coin.amount
            })
        })
        .collect()
}

/// Build an `AuthInfo` structure for signing a Cosmos transaction.
///
/// # Arguments
///
/// * `public_key_bytes` - A byte slice representing the compressed secp256k1 public key (33 bytes).
/// * `sequence` - The account sequence number for the signer.
/// * `fee` - A reference to the transaction fee.
///
/// # Returns
///
/// Returns `Ok(AuthInfo)` on success or an `Err(String)` describing the failure if the public key
/// bytes are invalid.
///
/// # Errors
///
/// Returns an error if the provided public key bytes are not a valid secp256k1 public key.
pub fn build_auth_info(
    public_key_bytes: &[u8],
    sequence: u64,
    fee: &Fee,
) -> crate::Result<AuthInfo> {
    let verifying_key = VerifyingKey::from_sec1_bytes(public_key_bytes)
        .map_err(|e| crate::KmsError::Msg(format!("Invalid secp256k1 public key: {e}")))?;

    let cosmos_pubkey = CosmosSecp256k1PublicKey::from(&verifying_key);
    let signer_info = SignerInfo::single_direct(Some(cosmos_pubkey), sequence);

    Ok(AuthInfo {
        signer_infos: vec![signer_info],
        fee: fee.clone(),
    })
}

#[must_use]
pub fn append_signature_to_transaction(
    tx_json: &serde_json::Value,
    new_signature: serde_json::Value,
) -> Vec<serde_json::Value> {
    match tx_json.get("signatures") {
        Some(serde_json::Value::Array(existing)) => {
            let mut updated = existing.clone();
            updated.push(new_signature);
            updated
        }
        _ => vec![new_signature],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::{
        COSMOS_PUBLIC_KEY, COSMOS_SIGNATURE, COSMOS_TRANSACTION, COSMOS_TRANSACTION_HASH,
    };
    use crate::{
        config::ConfigBuilder, services::crypto_service::CryptoService, wasm_loader::WasmLoader,
    };
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD;
    use serde_json::{Value, json};

    #[tokio::test]
    async fn test_create_key() {
        let config = ConfigBuilder::new()
            .with_cosmos_mode()
            .with_aws_mode(false)
            .build();

        let wasm_loader = WasmLoader::new().await.expect("Failed to load WASM module");

        let crypto_service =
            CryptoService::new(&wasm_loader).expect("Failed to initialize CryptoService");

        // Create the service via the real constructor, it will use MockKmsClientService internally
        let mut service = CosmosKeysService::new(config.clone(), crypto_service)
            .await
            .expect("Failed to create CosmosKeysService");

        // Call create_key on the service under test
        let result = service.create_key(&config).await;

        // Assert the key is returned
        assert!(result.is_ok(), "create_key failed: {result:?}");
        let key = result.unwrap();
        assert_ne!(key.public_key.as_deref().unwrap().to_string(), "");
    }

    #[tokio::test]
    async fn test_verify_signature() {
        let config = ConfigBuilder::new()
            .with_cosmos_mode()
            .with_aws_mode(false)
            .build();

        let wasm_loader = WasmLoader::new().await.expect("Failed to load WASM module");

        let crypto_service =
            CryptoService::new(&wasm_loader).expect("Failed to initialize CryptoService");

        // Create CosmosKeysService (uses mocked KMS + real CryptoService)
        let mut service = CosmosKeysService::new(config.clone(), crypto_service)
            .await
            .expect("Failed to create CosmosKeysService");

        // Valid signature
        let result = service
            .verify(COSMOS_TRANSACTION_HASH, COSMOS_SIGNATURE, COSMOS_PUBLIC_KEY)
            .await
            .unwrap();
        assert!(result, "Expected signature to verify correctly");

        // Invalid signature
        let result = service
            .verify("bad_hash", "bad_signature", "bad_key")
            .await
            .unwrap();
        assert!(!result, "Expected signature verification to fail");
    }

    #[tokio::test]
    async fn test_verify_via_kms_signature() {
        let config = ConfigBuilder::new()
            .with_cosmos_mode()
            .with_aws_mode(false)
            .build();

        let wasm_loader = WasmLoader::new().await.expect("Failed to load WASM module");

        let crypto_service =
            CryptoService::new(&wasm_loader).expect("Failed to initialize CryptoService");

        // Create CosmosKeysService (uses mocked KMS + real CryptoService)
        let mut service = CosmosKeysService::new(config.clone(), crypto_service)
            .await
            .expect("Failed to create CosmosKeysService");

        // Valid signature
        let result = service
            .verify_via_kms(COSMOS_TRANSACTION_HASH, COSMOS_SIGNATURE, COSMOS_PUBLIC_KEY)
            .await
            .unwrap();
        assert!(result, "Expected signature to verify correctly");

        // Invalid signature
        let result = service
            .verify_via_kms("bad_hash", "bad_signature", "bad_key")
            .await
            .unwrap();
        assert!(!result, "Expected signature verification to fail");
    }

    #[tokio::test]
    async fn test_delete_key() {
        let config = ConfigBuilder::new()
            .with_cosmos_mode()
            .with_aws_mode(false)
            .build();

        let wasm_loader = WasmLoader::new().await.expect("Failed to load WASM module");

        let crypto_service =
            CryptoService::new(&wasm_loader).expect("Failed to initialize CryptoService");

        let mut service = CosmosKeysService::new(config.clone(), crypto_service)
            .await
            .expect("Failed to create CosmosKeysService");

        let result = service
            .delete_key("known_public_key")
            .await
            .expect("Failed to delete known public key");
        assert!(result, "Expected key to be deleted successfully");

        let result = service
            .delete_key("unknown_key")
            .await
            .expect("Failed to delete unknown public key");
        assert!(!result, "Expected key deletion to fail for unknown key");
    }

    #[tokio::test]
    async fn test_list_keys() {
        let config = ConfigBuilder::new()
            .with_cosmos_mode()
            .with_aws_mode(false)
            .build();

        let wasm_loader = WasmLoader::new().await.expect("Failed to load WASM module");

        let crypto_service =
            CryptoService::new(&wasm_loader).expect("Failed to initialize CryptoService");

        let mut service = CosmosKeysService::new(config.clone(), crypto_service)
            .await
            .expect("Failed to create CosmosKeysService");

        let result = service.list_keys().await;

        assert!(result.is_ok(), "Expected list_keys to succeed");

        let keys = result.unwrap();
        assert_eq!(
            keys[0],
            KeyEntry {
                address: "address_1".to_string().into(),
                public_key_base64: STANDARD.encode("public_key_1_base64").into(),
                public_key: Some("public_key_1".to_string()).into(),
                key_id: "key_id_1".to_string().into(),
            }
        );
        assert_eq!(
            keys[1],
            KeyEntry {
                address: "address_2".to_string().into(),
                public_key_base64: STANDARD.encode("public_key_2_base64").into(),
                public_key: Some("public_key_2".to_string()).into(),
                key_id: "key_id_2".to_string().into(),
            }
        );
    }

    #[tokio::test]
    async fn test_sign_transaction_hash() {
        let config = ConfigBuilder::new()
            .with_cosmos_mode()
            .with_aws_mode(false)
            .build();

        let wasm_loader = WasmLoader::new().await.expect("Failed to load WASM module");

        let crypto_service =
            CryptoService::new(&wasm_loader).expect("Failed to initialize CryptoService");

        let mut service = CosmosKeysService::new(config.clone(), crypto_service)
            .await
            .expect("Failed to create CosmosKeysService");

        let result = service
            .sign_transaction_hash(&config, COSMOS_TRANSACTION_HASH, COSMOS_PUBLIC_KEY)
            .await;

        assert!(result.is_ok(), "Expected signing to succeed");

        let signed = result.unwrap();

        let expected_sig_hex = COSMOS_SIGNATURE;

        assert_eq!(
            signed.signature.to_string(),
            expected_sig_hex,
            "Expected signature to match expected format"
        );
    }

    #[tokio::test]
    async fn test_sign_transaction() {
        let config = ConfigBuilder::new()
            .with_cosmos_mode()
            .with_aws_mode(false)
            .build();

        let wasm_loader = WasmLoader::new().await.expect("Failed to load WASM module");

        let crypto_service =
            CryptoService::new(&wasm_loader).expect("Failed to initialize CryptoService");

        // Create the CosmosKeysService with mocks + real crypto
        let mut service = CosmosKeysService::new(config.clone(), crypto_service)
            .await
            .expect("Failed to create CosmosKeysService");

        let result = service
            .sign_transaction(&config, COSMOS_TRANSACTION, COSMOS_PUBLIC_KEY)
            .await;

        assert!(result.is_ok(), "Expected signing to succeed");

        let signed_tx = result.unwrap();
        let json: Value = serde_json::from_str(&signed_tx).expect("Invalid JSON returned");

        let signatures = json["signatures"]
            .as_array()
            .expect("Missing 'signatures' array");
        let first = &signatures[0];

        let signer_key = first["signer"].as_str().expect("Missing 'signer'");
        let signature = first["signature"].as_str().expect("Missing 'signature'");

        assert_eq!(
            signer_key, COSMOS_PUBLIC_KEY,
            "Expected signer to match COSMOS_PUBLIC_KEY"
        );

        assert_eq!(
            signature, COSMOS_SIGNATURE,
            "Expected signature to match expected format"
        );
    }

    #[tokio::test]
    async fn test_sign_transaction_malformed_json() {
        let config = ConfigBuilder::new()
            .with_cosmos_mode()
            .with_aws_mode(false)
            .build();

        let wasm_loader = WasmLoader::new().await.expect("Failed to load WASM");

        let crypto_service =
            CryptoService::new(&wasm_loader).expect("Failed to create CryptoService");

        let mut service = CosmosKeysService::new(config.clone(), crypto_service)
            .await
            .expect("Failed to create service");

        let bad_json = "{ this is not valid JSON }";

        let result = service
            .sign_transaction(&config, bad_json, COSMOS_PUBLIC_KEY)
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("Failed to parse JSON")
                || err.contains("Failed to parse cosmos transaction"),
            "Expected parsing error, got: {err}"
        );
    }

    #[tokio::test]
    async fn test_sign_transaction_missing_transaction_field() {
        let config = ConfigBuilder::new()
            .with_cosmos_mode()
            .with_aws_mode(false)
            .build();

        let wasm_loader = WasmLoader::new().await.expect("Failed to load WASM");

        let crypto_service =
            CryptoService::new(&wasm_loader).expect("Failed to create CryptoService");

        let mut service = CosmosKeysService::new(config.clone(), crypto_service)
            .await
            .expect("Failed to create service");

        let minimal_transaction = json!({
            "some": "value"
        })
        .to_string();

        let result = service
            .sign_transaction(&config, &minimal_transaction, COSMOS_PUBLIC_KEY)
            .await;

        assert!(
            result.is_err(),
            "Expected failure due to missing transaction fields"
        );
    }

    #[tokio::test]
    async fn test_sign_transaction_hash_with_invalid_input() {
        let config = ConfigBuilder::new()
            .with_cosmos_mode()
            .with_aws_mode(false)
            .build();

        let wasm_loader = WasmLoader::new().await.expect("Failed to load WASM module");

        let crypto_service =
            CryptoService::new(&wasm_loader).expect("Failed to initialize CryptoService");

        let mut service = CosmosKeysService::new(config.clone(), crypto_service)
            .await
            .expect("Failed to create CosmosKeysService");

        // Invalid public key and transaction hash
        let result = service
            .sign_transaction_hash(&config, "invalid_hash", "invalid_key")
            .await;

        assert!(
            result.is_err(),
            "Expected signing to fail due to invalid input"
        );
        let error = result.unwrap_err().to_string();
        assert!(
            error.contains("Invalid transaction hash hex"),
            "Unexpected error: {error}"
        );
    }

    #[tokio::test]
    async fn test_sign_transaction_malformed_approvals() {
        let config = ConfigBuilder::new()
            .with_cosmos_mode()
            .with_aws_mode(false)
            .build();

        let wasm_loader = WasmLoader::new().await.expect("Failed to load WASM");

        let crypto_service =
            CryptoService::new(&wasm_loader).expect("Failed to create CryptoService");

        let mut service = CosmosKeysService::new(config.clone(), crypto_service)
            .await
            .expect("Failed to create service");

        // approvals should be an array, here it's a string (malformed)
        let transaction = json!({
            "hash": COSMOS_TRANSACTION_HASH,
            "approvals": "not an array"
        })
        .to_string();

        let result = service
            .sign_transaction(&config, &transaction, COSMOS_PUBLIC_KEY)
            .await;

        assert!(
            result.is_err(),
            "Expected failure due to malformed approvals field"
        );
    }
}
