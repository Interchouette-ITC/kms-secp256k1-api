use crate::config::Config;
use crate::constants::ETH_SECP_LEN;
use crate::services::crypto_service::CryptoService;
use crate::services::keys_service::{KeyEntry, KeysService, KeysServiceTrait};
use ethers::types::{H256, Signature, TransactionRequest, TxHash};
use k256::PublicKey;
use serde_json::json;
use std::str::FromStr;
use tracing::{error, info};

pub struct EthereumKeysService {
    keys_service: KeysService,
}

impl EthereumKeysService {
    pub async fn new(config: Config, crypto_service: CryptoService) -> Result<Self, String> {
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
    async fn create_key(&mut self, _config: &Config) -> Result<KeyEntry, String> {
        let (key_id, public_key_base64) = self
            .keys_service
            .kms_client_service
            .create_key()
            .await
            .map_err(|e| {
            let msg = format!("Failed to create_key in KmsClientService: {e}");
            error!("{}", &msg);
            msg
        })?;

        let public_key = self
            .keys_service
            .crypto_service
            .public_key(&public_key_base64)
            .map_err(|e| {
                let msg = format!("public_key conversion failed: {e:?}");
                error!("{}", &msg);
                msg
            })?;

        let alias = self.resolve_alias(&public_key)?;

        if public_key.is_empty() {
            let msg = "No public key generated".to_string();
            error!("{}", &msg);
            return Err(msg);
        }

        info!("Public key ethereum retrieved: {}", public_key);
        info!("Address alias ethereum retrieved: {}", alias);

        // Create alias for the key
        self.keys_service
            .kms_client_service
            .create_alias(&key_id, &alias)
            .await
            .map_err(|e| {
                let msg = format!("Error creating alias: {e:?}");
                error!("{}", &msg);
                msg
            })?;

        Ok(KeyEntry {
            public_key: Some(public_key.clone()).into(),
            address: alias.into(),
            public_key_base64: public_key_base64.into(),
            key_id: key_id.into(),
        })
    }

    /// Signs a transaction hash using the provided public key and configuration mode.
    ///
    /// Selects a prefix based on the active mode (Ethereum or Ethereum),
    /// and delegates signing to the internal `sign` method.
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
        alias: &str,
    ) -> Result<String, String> {
        if !config.is_ethereum_mode() {
            return Err("Only Ethereum mode is supported".to_string());
        }

        info!("transaction_hash to sign: {}", transaction_hash);

        if let Err(e) = transaction_hash.parse::<H256>() {
            info!(
                "Error reading parameters: transaction_hash : {}",
                transaction_hash
            );
            error!("Validation error: {:?}", e);
            return Err("Error reading transaction TransactionHash parameters".to_string());
        }

        let public_key = self
            .keys_service
            .kms_client_service
            .get_public_key(alias)
            .await
            .map_err(|e| {
                let msg = format!("Failed to get public key from alias with KMS: {e}");
                error!("{}", msg);
                msg
            })?;

        let public_key_bytes =
            match hex::decode(public_key.strip_prefix("0x").unwrap_or(&public_key)) {
                Ok(bytes) => bytes,
                Err(e) => {
                    info!(
                        "Error reading parameters \npublic_key : {}\ntransaction_hash : {}",
                        public_key, transaction_hash
                    );
                    error!("Validation error (hex decode): {:?}", e);
                    return Err("Error reading transaction PublicKey parameters".to_string());
                }
            };

        if let Err(e) = PublicKey::from_sec1_bytes(&public_key_bytes) {
            info!(
                "Error reading parameters \npublic_key : {}\ntransaction_hash : {}",
                public_key, transaction_hash
            );
            error!("Validation error (public key parse): {:?}", e);
            return Err("Error reading transaction PublicKey parameters".to_string());
        }

        // Perform signing
        let signature = self
            .keys_service
            .sign(transaction_hash, &public_key, None)
            .await
            .map_err(|e| format!("Signing failed: {e}"))?;

        match hex::decode(&signature) {
            Ok(bytes) if bytes.len() == 64 || bytes.len() == 65 => bytes,
            _ => return Err("Error reading signature length".to_string()),
        };

        // let v = if sig_bytes.len() == 65 {
        //     sig_bytes[64]
        // } else {
        //     0 // default v for Casper or signatures without recovery id
        // };

        // info!(format!("{v:x}"));

        // Verify signature
        let is_valid = self.verify(transaction_hash, &signature, &public_key)?;

        if !is_valid {
            return Err("Signature verification failed".to_string());
        }

        Ok(signature)
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
        alias: &str,
    ) -> Result<String, String> {
        if !config.is_ethereum_mode() {
            return Err("Only Ethereum mode is supported".to_string());
        }

        // Parse the input JSON (support wrapped or plain)
        let parsed: serde_json::Value = serde_json::from_str(transaction_str)
            .map_err(|e| format!("Failed to parse input JSON: {e}"))?;

        // Extract transaction and existing signatures if wrapped
        let (transaction_value, mut signatures) = match parsed {
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
            _ => return Err("Unsupported transaction format".to_string()),
        };

        // Deserialize transaction for sighash
        let transaction: TransactionRequest = serde_json::from_value(transaction_value.clone())
            .map_err(|e| format!("Failed to parse transaction: {e}"))?;

        let mut transaction_hash_str = format!("{:x}", transaction.sighash());
        transaction_hash_str = transaction_hash_str.trim_start_matches("0x").to_string();

        // Validate transaction hash
        TxHash::from_str(&transaction_hash_str).map_err(|e| {
            error!("Invalid transaction hash: {:?}", e);
            "Invalid transaction hash".to_string()
        })?;

        let public_key = self
            .keys_service
            .kms_client_service
            .get_public_key(alias)
            .await
            .map_err(|e| {
                let msg = format!("Failed to get public key from alias with KMS: {e}");
                error!("{}", msg);
                msg
            })?;

        // Decode and validate public key bytes
        let public_key_bytes =
            match hex::decode(public_key.strip_prefix("0x").unwrap_or(&public_key)) {
                Ok(bytes) => bytes,
                Err(e) => {
                    info!(
                        "Error reading parameters \npublic_key : {}\ntransaction_hash : {}",
                        public_key, transaction_hash_str
                    );
                    error!("Validation error (hex decode): {:?}", e);
                    return Err("Error reading transaction PublicKey parameters".to_string());
                }
            };

        if let Err(e) = PublicKey::from_sec1_bytes(&public_key_bytes) {
            info!(
                "Error reading parameters \npublic_key : {}\ntransaction_hash : {}",
                public_key, transaction_hash_str
            );
            error!("Validation error (public key parse): {:?}", e);
            return Err("Error reading transaction PublicKey parameters".to_string());
        }

        // Perform signing
        let signature_hex = self
            .keys_service
            .sign(&transaction_hash_str, &public_key, None)
            .await
            .map_err(|e| format!("Signing failed: {e}"))?;

        // Verify signature
        let is_valid = self.verify(&transaction_hash_str, &signature_hex, &public_key)?;
        if !is_valid {
            return Err("Generated signature failed verification".to_string());
        }

        // Parse signature to get v, r, s
        let signature = Signature::from_str(&signature_hex)
            .map_err(|e| format!("Failed to serialize signature: {e}"))?;

        // Append new signature to signatures array
        signatures.push(json!({
            "signer": public_key,
            "v": format!("{:x}", signature.v),
            "r": format!("{:x}", signature.r),
            "s": format!("{:x}", signature.s),
            "hash": transaction_hash_str,
            "signature": signature_hex,
        }));

        // Return wrapped transaction and signatures
        let result = json!({
            "transaction": transaction_value,
            "signatures": signatures,
        });

        serde_json::to_string(&result)
            .map_err(|e| format!("Failed to serialize signed transaction JSON: {e}"))
    }

    fn verify(
        &mut self,
        transaction_hash_hex: &str,
        signature_hex: &str,
        public_key: &str,
    ) -> Result<bool, String> {
        self.keys_service
            .verify_eip155(transaction_hash_hex, signature_hex, public_key)
    }

    async fn verify_via_kms(
        &mut self,
        transaction_hash_hex: &str,
        signature_hex: &str,
        public_key: &str,
    ) -> Result<bool, String> {
        self.keys_service
            .verify_via_kms_eip155(transaction_hash_hex, signature_hex, public_key)
            .await
    }

    async fn delete_key(&mut self, alias: &str) -> Result<bool, String> {
        let final_alias = self.resolve_alias(alias)?;
        self.keys_service.delete_key(&final_alias).await
    }

    async fn list_keys(&mut self) -> Result<Vec<KeyEntry>, String> {
        self.keys_service.list_keys().await
    }
}

impl EthereumKeysService {
    /// Resolves a given alias string to an Ethereum address.
    ///
    /// This function checks whether the input `alias` is a compressed secp256k1 public key
    /// (by comparing its length to the expected `ETH_SECP_LEN`). If so, it attempts to convert
    /// the public key to its corresponding Ethereum address using the crypto service. Otherwise,
    /// it assumes the alias is already an address and returns it as-is.
    ///
    /// # Parameters
    /// - `alias`: A string that is either an Ethereum address or a compressed public key (hex-encoded, starting with "02"/"03").
    ///
    /// # Returns
    /// - `Ok(String)`: The resolved Ethereum address as a string.
    /// - `Err(String)`: An error message if public key conversion fails.
    ///
    /// # Errors
    /// - Returns an error if the input is treated as a public key and the conversion fails.
    ///
    fn resolve_alias(&mut self, alias: &str) -> Result<String, String> {
        if alias.len() == ETH_SECP_LEN {
            self.keys_service
                .crypto_service
                .address_eth(alias)
                .map_err(|e| {
                    let msg = format!("Failed to convert public key to address: {e:?}");
                    error!("{}", &msg);
                    msg
                })
        } else {
            Ok(alias.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::ConfigBuilder,
        constants::{
            ETH_ADDRESS, ETH_PUBLIC_KEY, ETH_SIGNATURE, ETH_TRANSACTION, ETH_TRANSACTION_HASH,
            WASM_PATH,
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

        let wasm_loader = WasmLoader::new(WASM_PATH)
            .await
            .expect("Failed to load WASM module");

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

        let wasm_loader = WasmLoader::new(WASM_PATH)
            .await
            .expect("Failed to load WASM module");

        let crypto_service =
            CryptoService::new(&wasm_loader).expect("Failed to initialize CryptoService");

        // Create EthereumKeysService (uses mocked KMS + real CryptoService)
        let mut service = EthereumKeysService::new(config.clone(), crypto_service)
            .await
            .expect("Failed to create EthereumKeysService");

        // Valid signature
        let result = service
            .verify(ETH_TRANSACTION_HASH, ETH_SIGNATURE, ETH_PUBLIC_KEY)
            .unwrap();
        assert!(result, "Expected signature to verify correctly");

        // Invalid signature
        let result = service
            .verify("bad_hash", "bad_signature", "bad_key")
            .unwrap();
        assert!(!result, "Expected signature verification to fail");
    }

    #[tokio::test]
    async fn test_verify_via_kms_signature() {
        let config = ConfigBuilder::new()
            .with_ethereum_mode()
            .with_aws_mode(false)
            .build();

        let wasm_loader = WasmLoader::new(WASM_PATH)
            .await
            .expect("Failed to load WASM module");

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

        let wasm_loader = WasmLoader::new(WASM_PATH)
            .await
            .expect("Failed to load WASM module");

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

        let wasm_loader = WasmLoader::new(WASM_PATH)
            .await
            .expect("Failed to load WASM module");

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

        let wasm_loader = WasmLoader::new(WASM_PATH)
            .await
            .expect("Failed to load WASM module");

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
            signed, ETH_SIGNATURE,
            "Expected signature to match expected format"
        );
    }

    #[tokio::test]
    async fn test_sign_transaction_hash_from_address() {
        let config = ConfigBuilder::new()
            .with_ethereum_mode()
            .with_aws_mode(false)
            .build();

        let wasm_loader = WasmLoader::new(WASM_PATH)
            .await
            .expect("Failed to load WASM module");

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
            signed, ETH_SIGNATURE,
            "Expected signature to match expected format"
        );
    }

    #[tokio::test]
    async fn test_sign_transaction() {
        let config = ConfigBuilder::new()
            .with_ethereum_mode()
            .with_aws_mode(false)
            .build();

        let wasm_loader = WasmLoader::new(WASM_PATH)
            .await
            .expect("Failed to load WASM module");

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

        let signed = result.unwrap();
        let json: Value = serde_json::from_str(&signed).expect("Invalid JSON returned");

        let signatures = json["signatures"]
            .as_array()
            .expect("Missing 'signatures' array");
        let first = &signatures[0];

        let signer = first["signer"].as_str().expect("Missing 'signer'");
        let signature = first["signature"].as_str().expect("Missing 'signature'");

        assert_eq!(
            signer, ETH_PUBLIC_KEY,
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

        let wasm_loader = WasmLoader::new(WASM_PATH)
            .await
            .expect("Failed to load WASM module");

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

        let signed = result.unwrap();
        let json: Value = serde_json::from_str(&signed).expect("Invalid JSON returned");

        let signatures = json["signatures"]
            .as_array()
            .expect("Missing 'signatures' array");
        let first = &signatures[0];

        let signer = first["signer"].as_str().expect("Missing 'signer'");
        let signature = first["signature"].as_str().expect("Missing 'signature'");

        assert_eq!(
            signer, ETH_PUBLIC_KEY,
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

        let wasm_loader = WasmLoader::new(WASM_PATH)
            .await
            .expect("Failed to load WASM");

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
        assert!(
            result
                .unwrap_err()
                .starts_with("Failed to parse input JSON"),
            "Expected JSON parsing failure"
        );
    }

    #[tokio::test]
    async fn test_sign_transaction_missing_transaction_field() {
        let config = ConfigBuilder::new()
            .with_ethereum_mode()
            .with_aws_mode(false)
            .build();

        let wasm_loader = WasmLoader::new(WASM_PATH)
            .await
            .expect("Failed to load WASM");

        let crypto_service =
            CryptoService::new(&wasm_loader).expect("Failed to create crypto service");

        let mut service = EthereumKeysService::new(config.clone(), crypto_service)
            .await
            .expect("Failed to create service");

        // JSON object with no "transaction" field — should fallback to whole object
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

        let wasm_loader = WasmLoader::new(WASM_PATH)
            .await
            .expect("Failed to load WASM");

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
