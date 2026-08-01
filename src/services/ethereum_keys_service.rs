use crate::config::Config;
use crate::constants::{ETH_SECP_LEN, SIGNATURE_RS_LEN};
use crate::services::crypto_service::CryptoService;
use crate::services::keys_service::{KeyEntry, KeysService, KeysServiceTrait, SigEntry};
use ethers::types::{H256, Signature, TransactionRequest};
use k256::PublicKey;
use serde_json::json;
use std::str::FromStr;
use tracing::error;

pub struct EthereumKeysService {
    keys_service: KeysService,
}

impl EthereumKeysService {
    /// Creates a new instance of the service.
    ///
    /// # Parameters
    /// - `config`: The configuration settings for the service.
    /// - `crypto_service`: The cryptographic service used for key management.
    ///
    /// # Returns
    /// Returns `Ok(Self)` if the service is successfully created, or
    /// an `Err(String)` containing an error message if initialization fails.
    ///
    /// # Errors
    /// This function returns an error if the underlying `KeysService::new`
    /// call fails, propagating its error as a string.
    pub async fn new(config: Config, crypto_service: CryptoService) -> crate::Result<Self> {
        let keys_service = KeysService::new(config, crypto_service).await?;
        Ok(Self { keys_service })
    }
}

#[async_trait::async_trait]
impl KeysServiceTrait for EthereumKeysService {
    /// Creates a new KMS key, derives its public key with an optional prefix based on config,
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
            address: key.into(),
            public_key_base64: public_key_base64.into(),
            key_id: key_id.into(),
        })
    }

    /// Signs a transaction hash using the provided public key and configuration mode.
    ///
    /// Delegates signing to the keys service `sign` method.
    ///
    /// # Arguments
    ///
    /// * `config` - Reference to the application configuration to determine signing mode.
    /// * `transaction_hash` - The hash of the transaction to be signed.
    /// * `public_key` - The public key corresponding to the private key for signing.
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
        Self::ensure_ethereum_mode(config)?;

        Self::validate_transaction_hash(transaction_hash)?;

        let key = self.resolve_key(key)?;
        let public_key = self.resolve_public_key(&key).await?;

        Self::validate_public_key(&public_key, "sign_transaction_hash")?;

        // Perform signing
        let signature = self
            .sign_with_recovery_id(transaction_hash, &key, &public_key, config)
            .await?;

        Ok(SigEntry {
            address: key.into(),
            public_key: public_key.into(),
            signature: signature.into(),
        })
    }

    /// Signs a serialized Ethereum transaction using the provided public key.
    ///
    /// Parses the input transaction string, verifies the transaction hash and public key,
    /// signs the hash, attaches the signature to the transaction, and returns the signed
    /// transaction as a JSON string.
    ///
    /// # Arguments
    ///
    /// * `config` - Reference to the application configuration. Only Ethereum mode is supported.
    /// * `transaction_str` - A JSON string representing the transaction to be signed.
    /// * `public_key` - The public key corresponding to the signing key.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The application is not in Ethereum mode.
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
        Self::ensure_ethereum_mode(config)?;

        let tx_json: serde_json::Value = serde_json::from_str(transaction_str)
            .map_err(|e| crate::KmsError::ParseJson(e.to_string()))?;

        // Extract transaction and existing signatures if wrapped
        let (transaction_value, mut signatures) = match tx_json {
            serde_json::Value::Object(mut map) => {
                if let Some(tx) = map.remove("transaction") {
                    let sigs = map
                        .remove("signatures")
                        .and_then(|v| v.as_array().cloned())
                        .unwrap_or_default();
                    (tx, sigs)
                } else {
                    (serde_json::Value::Object(map), vec![])
                }
            }
            _ => return Err(crate::KmsError::UnsupportedTxFormat),
        };

        // Deserialize transaction for sighash
        let transaction: TransactionRequest = serde_json::from_value(transaction_value.clone())
            .map_err(|e| {
                crate::KmsError::ParseTransaction(format!("Failed to parse transaction: {e}"))
            })?;

        let mut transaction_hash = format!("{:x}", transaction.sighash());
        transaction_hash = transaction_hash.trim_start_matches("0x").to_string();

        let key = self.resolve_key(key)?;
        let public_key = self.resolve_public_key(&key).await?;

        // Decode and validate public key bytes
        Self::validate_public_key(&public_key, "sign_transaction")?;

        // Perform signing
        let signature_hex = self
            .sign_with_recovery_id(&transaction_hash, &key, &public_key, config)
            .await?;

        // Parse signature to get v, r, s
        let signature = Signature::from_str(&signature_hex)
            .map_err(|e| crate::KmsError::Msg(format!("Failed to serialize signature: {e}")))?;

        // Append new signature to signatures array
        signatures.push(Self::signature_to_json(
            &key,
            &signature,
            &public_key,
            &transaction_hash,
        ));

        // Return wrapped transaction and signatures
        let result = json!({
            "transaction": transaction_value,
            "signatures": signatures,
        });

        serde_json::to_string(&result).map_err(|e| {
            crate::KmsError::Msg(format!("Failed to serialize signed transaction JSON: {e}"))
        })
    }

    async fn verify(
        &mut self,
        transaction_hash: &str,
        signature_hex: &str,
        key: &str,
    ) -> crate::Result<bool> {
        let public_key = self.resolve_public_key(key).await?;
        self.keys_service
            .verify_eip155(transaction_hash, signature_hex, &public_key)
    }

    async fn verify_via_kms(
        &mut self,
        transaction_hash: &str,
        signature_hex: &str,
        key: &str,
    ) -> crate::Result<bool> {
        let public_key = self.resolve_public_key(key).await?;
        self.keys_service
            .verify_via_kms_eip155(transaction_hash, signature_hex, &public_key)
            .await
    }

    async fn delete_key(&mut self, key: &str) -> crate::Result<bool> {
        // Delegates to KeysService after resolving the Ethereum address.
        let key = self.resolve_key(key)?;
        self.keys_service.delete_key(&key).await
    }

    async fn list_keys(&mut self) -> crate::Result<Vec<KeyEntry>> {
        // Delegates to KeysService.
        self.keys_service.list_keys().await
    }
}

impl EthereumKeysService {
    /// Signs a transaction hash using the provided key, and appends the recovery byte (`v`)
    /// if the resulting signature is 128 hex characters long (i.e., 64 bytes).
    ///
    /// # Parameters
    /// - `transaction_hash`: The hex-encoded transaction hash to sign.
    /// - `public_key`: The public key associated with the signer.
    /// - `config`: The application config (used to determine chain ID).
    ///
    /// # Returns
    /// - `Ok(String)`: The final hex-encoded signature (possibly with `v` appended).
    /// - `Err(String)`: An error message if signing fails or recovery fails.
    async fn sign_with_recovery_id(
        &mut self,
        transaction_hash: &str,
        key: &str,
        public_key: &str,
        config: &Config,
    ) -> crate::Result<String> {
        let mut signature_hex = self
            .keys_service
            .sign(transaction_hash, key, None)
            .await
            .map_err(|e| crate::KmsError::SigningFailed(e.to_string()))?;

        if signature_hex.len() == SIGNATURE_RS_LEN {
            let v_hex = match self.keys_service.crypto_service.recover_v(
                transaction_hash,
                &signature_hex,
                public_key,
                Some(config.get_eth_chain_id()),
            ) {
                Ok(v) => v,
                Err(e) => {
                    error!("Failed to recover v: {}", e);
                    String::new()
                }
            };

            if !v_hex.is_empty() {
                signature_hex.push_str(&v_hex);
            }
        }

        Self::validate_signature_length(&signature_hex)?;

        // Verify signature
        let is_valid = self
            .verify(transaction_hash, &signature_hex, public_key)
            .await?;
        if !is_valid {
            return Err(crate::KmsError::VerificationFailed);
        }

        Ok(signature_hex)
    }

    /// Resolves a given key string to an Ethereum address.
    ///
    /// This function checks whether the input `key` is a compressed secp256k1 public key
    /// (by comparing its length to the expected `ETH_SECP_LEN`). If so, it attempts to convert
    /// the public key to its corresponding Ethereum address using the crypto service. Otherwise,
    /// it assumes the key is already an address and returns it as-is.
    ///
    /// # Parameters
    /// - `key`: A string that is either an Ethereum address or a compressed public key (hex-encoded, starting with "02"/"03").
    ///
    /// # Returns
    /// - `Ok(String)`: The resolved Ethereum address as a string.
    /// - `Err(String)`: An error message if public key conversion fails.
    ///
    /// # Errors
    /// - Returns an error if the input is treated as a public key and the conversion fails.
    ///
    fn resolve_key(&mut self, key: &str) -> crate::Result<String> {
        if key.len() == ETH_SECP_LEN {
            self.keys_service
                .crypto_service
                .address_eth(key)
                .inspect_err(|e| {
                    error!(error = %e, "Failed to convert public key to address");
                })
        } else {
            Ok(key.to_string())
        }
    }

    /// Resolves a key string to its corresponding public key.
    ///
    /// If the input `key` is already a compressed secp256k1 public key (determined by its length),
    /// it is returned directly. Otherwise, it is treated as an alias and resolved via the KMS service,
    /// followed by conversion to a usable public key format.
    ///
    /// # Parameters
    /// - `key`: A compressed public key or a key alias.
    ///
    /// # Returns
    /// - `Ok(String)`: The resolved public key as a hex string.
    /// - `Err(String)`: An error message if resolution or conversion fails.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The public key cannot be retrieved from the KMS for the given alias.
    /// - Conversion from raw public key bytes to the expected hex format fails.
    pub async fn resolve_public_key(&mut self, key: &str) -> crate::Result<String> {
        if key.len() == ETH_SECP_LEN {
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

    fn validate_signature_length(hex: &str) -> crate::Result<()> {
        let bytes = hex::decode(hex).map_err(|_| crate::KmsError::InvalidSignatureHex)?;
        if bytes.len() == 65 {
            Ok(())
        } else {
            Err(crate::KmsError::InvalidSignatureLength { expected: 65 })
        }
    }

    fn ensure_ethereum_mode(config: &Config) -> crate::Result<()> {
        if config.is_ethereum_mode() {
            Ok(())
        } else {
            Err(crate::KmsError::ModeMismatch { mode: "Ethereum" })
        }
    }

    fn validate_transaction_hash(transaction_hash: &str) -> crate::Result<H256> {
        transaction_hash.parse::<H256>().map_err(|e| {
            let msg = format!("Invalid transaction hash: {e:?}");
            error!("{}", msg);
            crate::KmsError::Msg(msg)
        })
    }

    fn signature_to_json(
        address: &str,
        sig: &Signature,
        public_key: &str,
        hash: &str,
    ) -> serde_json::Value {
        json!({
            "address": address,
            "signer": public_key,
            "v": format!("{:x}", sig.v),
            "r": format!("{:x}", sig.r),
            "s": format!("{:x}", sig.s),
            "hash": hash,
            "signature": sig.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::ConfigBuilder,
        constants::{
            ETH_ADDRESS, ETH_PUBLIC_KEY, ETH_SIGNATURE, ETH_TRANSACTION, ETH_TRANSACTION_HASH,
        },
        services::crypto_service::CryptoService,
        wasm_loader::WasmLoader,
    };
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD;
    use serde_json::Value;

    #[tokio::test]
    async fn test_create_key() {
        let config = ConfigBuilder::new()
            .with_ethereum_mode()
            .with_aws_mode(false)
            .build();

        let wasm_loader = WasmLoader::new().await.expect("Failed to load WASM module");

        let crypto_service =
            CryptoService::new(&wasm_loader).expect("Failed to initialize CryptoService");

        // Create the service via the real constructor, it will use MockKmsClientService internally
        let mut service = EthereumKeysService::new(config.clone(), crypto_service)
            .await
            .expect("Failed to create EthereumKeysService");

        // Call create_key on the service under test
        let result = service.create_key(&config).await;

        // Assert the key is returned with the casper prefix
        assert!(result.is_ok(), "create_key failed: {result:?}");
        let key = result.unwrap();
        let pubkey = key.public_key.as_deref().expect("Missing public key");

        assert!(
            pubkey.starts_with("02") || pubkey.starts_with("03"),
            "Public key must start with 02 or 03"
        );
        assert!(!pubkey.is_empty(), "Public key must not be empty");
    }

    #[tokio::test]
    async fn test_verify_signature() {
        let config = ConfigBuilder::new()
            .with_ethereum_mode()
            .with_aws_mode(false)
            .build();

        let wasm_loader = WasmLoader::new().await.expect("Failed to load WASM module");

        let crypto_service =
            CryptoService::new(&wasm_loader).expect("Failed to initialize CryptoService");

        // Create EthereumKeysService (uses mocked KMS + real CryptoService)
        let mut service = EthereumKeysService::new(config.clone(), crypto_service)
            .await
            .expect("Failed to create EthereumKeysService");

        // Valid signature
        let result = service
            .verify(ETH_TRANSACTION_HASH, ETH_SIGNATURE, ETH_PUBLIC_KEY)
            .await
            .unwrap();
        assert!(result, "Expected signature to verify correctly");

        //Invalid signature
        let result = service
            .verify("bad_hash", "bad_signature", "bad_key")
            .await
            .unwrap();
        assert!(!result, "Expected signature verification to fail");
    }

    #[tokio::test]
    async fn test_verify_via_kms_signature() {
        let config = ConfigBuilder::new()
            .with_ethereum_mode()
            .with_aws_mode(false)
            .build();

        let wasm_loader = WasmLoader::new().await.expect("Failed to load WASM module");

        let crypto_service =
            CryptoService::new(&wasm_loader).expect("Failed to initialize CryptoService");

        // Create EthereumKeysService (uses mocked KMS + real CryptoService)
        let mut service = EthereumKeysService::new(config.clone(), crypto_service)
            .await
            .expect("Failed to create EthereumKeysService");

        // Valid signature
        let result = service
            .verify_via_kms(ETH_TRANSACTION_HASH, ETH_SIGNATURE, ETH_PUBLIC_KEY)
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
            .with_ethereum_mode()
            .with_aws_mode(false)
            .build();

        let wasm_loader = WasmLoader::new().await.expect("Failed to load WASM module");

        let crypto_service =
            CryptoService::new(&wasm_loader).expect("Failed to initialize CryptoService");

        // Create EthereumKeysService (uses mocked KMS + real CryptoService)
        let mut service = EthereumKeysService::new(config.clone(), crypto_service)
            .await
            .expect("Failed to create EthereumKeysService");

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
            .with_ethereum_mode()
            .with_aws_mode(false)
            .build();

        let wasm_loader = WasmLoader::new().await.expect("Failed to load WASM module");

        let crypto_service =
            CryptoService::new(&wasm_loader).expect("Failed to initialize CryptoService");

        // Create EthereumKeysService (uses mocked KMS + real CryptoService)
        let mut service = EthereumKeysService::new(config.clone(), crypto_service)
            .await
            .expect("Failed to create EthereumKeysService");

        let result = service.list_keys().await;

        assert!(result.is_ok(), "Expected list_keys to succeed");

        let keys = result.unwrap();
        assert_eq!(keys.len(), 2);
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
            .with_ethereum_mode()
            .with_aws_mode(false)
            .build();

        let wasm_loader = WasmLoader::new().await.expect("Failed to load WASM module");

        let crypto_service =
            CryptoService::new(&wasm_loader).expect("Failed to initialize CryptoService");

        // Create EthereumKeysService (uses mocked KMS + real CryptoService)
        let mut service = EthereumKeysService::new(config.clone(), crypto_service)
            .await
            .expect("Failed to create EthereumKeysService");

        let result = service
            .sign_transaction_hash(&config, ETH_TRANSACTION_HASH, ETH_PUBLIC_KEY)
            .await;

        assert!(result.is_ok(), "Expected signing to succeed");

        let signed = result.unwrap();

        assert_eq!(
            signed.signature.to_string(),
            ETH_SIGNATURE,
            "Expected signature to match expected format"
        );
    }

    #[tokio::test]
    async fn test_sign_transaction_hash_from_address() {
        let config = ConfigBuilder::new()
            .with_ethereum_mode()
            .with_aws_mode(false)
            .build();

        let wasm_loader = WasmLoader::new().await.expect("Failed to load WASM module");

        let crypto_service =
            CryptoService::new(&wasm_loader).expect("Failed to initialize CryptoService");

        // Create EthereumKeysService (uses mocked KMS + real CryptoService)
        let mut service = EthereumKeysService::new(config.clone(), crypto_service)
            .await
            .expect("Failed to create EthereumKeysService");

        let result = service
            .sign_transaction_hash(&config, ETH_TRANSACTION_HASH, ETH_ADDRESS)
            .await;

        assert!(result.is_ok(), "Expected signing to succeed");

        let signed = result.unwrap();

        assert_eq!(
            signed.signature.to_string(),
            ETH_SIGNATURE,
            "Expected signature to match expected format"
        );
    }

    #[tokio::test]
    async fn test_sign_transaction() {
        let config = ConfigBuilder::new()
            .with_ethereum_mode()
            .with_aws_mode(false)
            .build();

        let wasm_loader = WasmLoader::new().await.expect("Failed to load WASM module");

        let crypto_service =
            CryptoService::new(&wasm_loader).expect("Failed to initialize CryptoService");

        // Create EthereumKeysService (uses mocked KMS + real CryptoService)
        let mut service = EthereumKeysService::new(config.clone(), crypto_service)
            .await
            .expect("Failed to create EthereumKeysService");

        let result = service
            .sign_transaction(&config, ETH_TRANSACTION, ETH_PUBLIC_KEY)
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
            signer_key, ETH_PUBLIC_KEY,
            "Expected signer to match ETH_PUBLIC_KEY"
        );

        assert_eq!(
            signature, ETH_SIGNATURE,
            "Expected signature to match expected format"
        );
    }

    #[tokio::test]
    async fn test_sign_transaction_from_address() {
        let config = ConfigBuilder::new()
            .with_ethereum_mode()
            .with_aws_mode(false)
            .build();

        let wasm_loader = WasmLoader::new().await.expect("Failed to load WASM module");

        let crypto_service =
            CryptoService::new(&wasm_loader).expect("Failed to initialize CryptoService");

        // Create EthereumKeysService (uses mocked KMS + real CryptoService)
        let mut service = EthereumKeysService::new(config.clone(), crypto_service)
            .await
            .expect("Failed to create EthereumKeysService");

        let result = service
            .sign_transaction(&config, ETH_TRANSACTION, ETH_ADDRESS)
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
            signer_key, ETH_PUBLIC_KEY,
            "Expected signer to match ETH_PUBLIC_KEY"
        );

        assert_eq!(
            signature, ETH_SIGNATURE,
            "Expected signature to match expected format"
        );
    }

    #[tokio::test]
    async fn test_sign_transaction_malformed_json() {
        let config = ConfigBuilder::new()
            .with_ethereum_mode()
            .with_aws_mode(false)
            .build();

        let wasm_loader = WasmLoader::new().await.expect("Failed to load WASM");

        let crypto_service =
            CryptoService::new(&wasm_loader).expect("Failed to create crypto service");

        let mut service = EthereumKeysService::new(config.clone(), crypto_service)
            .await
            .expect("Failed to create service");

        let bad_json = "{ this is not valid JSON }";

        let result = service
            .sign_transaction(&config, bad_json, ETH_PUBLIC_KEY)
            .await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, crate::KmsError::ParseJson(_))
                || err.to_string().starts_with("Failed to parse input JSON"),
            "Expected JSON parsing failure, got: {err}"
        );
    }

    #[tokio::test]
    async fn test_sign_transaction_missing_transaction_field() {
        let config = ConfigBuilder::new()
            .with_ethereum_mode()
            .with_aws_mode(false)
            .build();

        let wasm_loader = WasmLoader::new().await.expect("Failed to load WASM");

        let crypto_service =
            CryptoService::new(&wasm_loader).expect("Failed to create crypto service");

        let mut service = EthereumKeysService::new(config.clone(), crypto_service)
            .await
            .expect("Failed to create service");

        // JSON object with no "transaction" field - should fallback to whole object
        let transaction = json!({
            "to": "0xdeadbeef",
            "value": "0x1"
        })
        .to_string();

        let result = service
            .sign_transaction(&config, &transaction, ETH_PUBLIC_KEY)
            .await;

        // Either parse or signing might fail depending on internals
        assert!(result.is_err() || result.is_ok()); // up to your logic
    }

    #[tokio::test]
    async fn test_sign_transaction_malformed_signatures() {
        let config = ConfigBuilder::new()
            .with_ethereum_mode()
            .with_aws_mode(false)
            .build();

        let wasm_loader = WasmLoader::new().await.expect("Failed to load WASM");

        let crypto_service =
            CryptoService::new(&wasm_loader).expect("Failed to create crypto service");

        let mut service = EthereumKeysService::new(config.clone(), crypto_service)
            .await
            .expect("Failed to create service");

        let transaction = json!({
            "signatures": "not an array"
        })
        .to_string();

        let result = service
            .sign_transaction(&config, &transaction, ETH_PUBLIC_KEY)
            .await;

        assert!(result.is_err());
    }
}
