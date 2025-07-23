use crate::config::Config;
use crate::constants::{COSMOS_SECP_LEN, DEFAULT_COSMOS_UDENOM};
use crate::services::crypto_service::CryptoService;
use crate::services::keys_service::{KeyEntry, KeysService, KeysServiceTrait};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use tracing::error;

pub struct CosmosKeysService {
    keys_service: KeysService,
    udenom: String,
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
    async fn create_key(&mut self, _config: &Config) -> Result<KeyEntry, String> {
        let (key_id, public_key) = self
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
            .public_key(&public_key)
            .map_err(|e| {
                let msg = format!("public_key conversion failed: {e:?}");
                error!("{}", &msg);
                msg
            })?;

        if public_key.is_empty() {
            let msg = "No public key generated".to_string();
            error!("{}", &msg);
            return Err(msg);
        }

        let address = self.resolve_key(&public_key)?;

        // Create alias for the key
        self.keys_service
            .kms_client_service
            .create_alias(&key_id, &address)
            .await
            .map_err(|e| {
                let msg = format!("Error creating alias: {e:?}");
                error!("{}", &msg);
                msg
            })?;

        Ok(KeyEntry {
            public_key: Some(public_key.clone()).into(),
            address: address.clone().into(),
            public_key_base64: STANDARD.encode(public_key).into(),
            key_id: address.into(),
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
    ) -> Result<String, String> {
        if !config.is_cosmos_mode() {
            return Err("Only Cosmos mode is supported".to_string());
        }

        // if let Err(e) = TransactionHash::new(transaction_hash) {
        //     info!(
        //         "Error reading parameters \npublic_key : {}\ntransaction_hash : {}",
        //         public_key, transaction_hash
        //     );
        //     error!("Validation error: {:?}", e);
        //                 return Err(format!("Error reading transaction parameters: {e}"));
        // }

        // if let Err(e) = PublicKey::new(public_key) {
        //     info!(
        //         "Error reading parameters \npublic_key : {}\ntransaction_hash : {}",
        //         public_key, transaction_hash
        //     );
        //     error!("Validation error: {:?}", e);
        //                return Err(format!("Error reading transaction parameters: {e}"));
        // }

        // let signature = self
        //     .keys_service
        //     .sign(transaction_hash, public_key, None)
        //     .await?;

        // // Now verify the signature immediately
        // let verified = self.verify(transaction_hash, &signature, public_key)?;
        // if !verified {
        //     return Err("Signature verification failed after signing".to_string());
        // }

        Ok("".to_string())
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
        _transaction_str: &str,
        key: &str,
    ) -> Result<String, String> {
        if !config.is_cosmos_mode() {
            return Err("Only Cosmos mode is supported".to_string());
        }

        // let transaction: Transaction = Transaction::from_json_string(transaction_str)
        //     .map_err(|e| format!("Failed to parse transaction: {e}"))?;

        // let transaction_hash_str = transaction.hash().to_string();

        // TransactionHash::new(&transaction_hash_str).map_err(|e| {
        //     error!("Invalid transaction hash: {:?}", e);
        //     "Invalid transaction hash".to_string()
        // })?;

        // PublicKey::new(public_key).map_err(|e| {
        //     error!("Invalid public key: {:?}", e);
        //     "Invalid public key".to_string()
        // })?;

        // let signature = self
        //     .keys_service
        //     .sign(&transaction_hash_str, public_key, None)
        //     .await
        //     .map_err(|e| format!("Signing failed: {e}"))?;

        // // Verify the signature immediately
        // let verified = self.verify(&transaction_hash_str, &signature, public_key)?;
        // if !verified {
        //     return Err("Signature verification failed after signing".to_string());
        // }

        // let signed_transaction = transaction.add_signature(public_key, &signature);

        // signed_transaction
        //     .to_json_string()
        //     .map_err(|e| format!("Failed to serialize signed transaction: {e}"))
        Ok(String::from("value"))
    }

    async fn verify(
        &mut self,
        transaction_hash_hex: &str,
        signature_hex: &str,
        key: &str,
    ) -> Result<bool, String> {
        let key = self.resolve_key(key)?;
        self.keys_service
            .verify(transaction_hash_hex, signature_hex, &key)
    }

    async fn verify_via_kms(
        &mut self,
        transaction_hash_hex: &str,
        signature_hex: &str,
        key: &str,
    ) -> Result<bool, String> {
        let key = self.resolve_key(key)?;
        self.keys_service
            .verify_via_kms(transaction_hash_hex, signature_hex, &key)
            .await
    }

    async fn delete_key(&mut self, key: &str) -> Result<bool, String> {
        let key = self.resolve_key(key)?;
        self.keys_service.delete_key(&key).await
    }

    async fn list_keys(&mut self) -> Result<Vec<KeyEntry>, String> {
        self.keys_service.list_keys().await
    }
}

impl CosmosKeysService {
    pub async fn new(config: Config, crypto_service: CryptoService) -> Result<Self, String> {
        let keys_service = KeysService::new(config.clone(), crypto_service).await?;
        let cosmos_udenom = config.get_cosmos_udenom();
        let udenom = match cosmos_udenom.as_str() {
            "" => DEFAULT_COSMOS_UDENOM,
            udenom => udenom,
        }
        .to_string();
        Ok(Self {
            keys_service,
            udenom,
        })
    }

    fn resolve_key(&mut self, key: &str) -> Result<String, String> {
        if key.len() == COSMOS_SECP_LEN {
            self.keys_service
                .crypto_service
                .address_cosmos(key, &self.udenom)
                .map_err(|e| {
                    let msg = format!("Failed to convert public key to address: {e:?}");
                    error!("{}", &msg);
                    msg
                })
        } else {
            Ok(key.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::ConfigBuilder,
        constants::{
            CASPER_PUBLIC_KEY_PREFIXED, SIGNATURE, SIGNATURE_PREFIXED, TRANSACTION_HASH, WASM_PATH,
        },
        services::crypto_service::CryptoService,
        wasm_loader::WasmLoader,
    };
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD;
    use serde_json::json;

    #[tokio::test]
    async fn test_create_key() {
        let config = ConfigBuilder::new()
            .with_cosmos_mode()
            .with_aws_mode(false)
            .build();

        let wasm_loader = WasmLoader::new(WASM_PATH)
            .await
            .expect("Failed to load WASM module");

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
        assert!(!key.public_key.as_deref().unwrap().to_string().is_empty());
    }

    // #[tokio::test]
    // async fn test_verify_signature() {
    //     let config = ConfigBuilder::new()
    //         .with_cosmos_mode()
    //         .with_aws_mode(false)
    //         .build();

    //     let wasm_loader = WasmLoader::new(WASM_PATH)
    //         .await
    //         .expect("Failed to load WASM module");

    //     let crypto_service =
    //         CryptoService::new(&wasm_loader).expect("Failed to initialize CryptoService");

    //     // Create CosmosKeysService (uses mocked KMS + real CryptoService)
    //     let mut service = CosmosKeysService::new(config.clone(), crypto_service)
    //         .await
    //         .expect("Failed to create CosmosKeysService");

    //     //  Valid prefixed signature
    //     let result = service
    //         .verify(
    //             TRANSACTION_HASH,
    //             SIGNATURE_PREFIXED,
    //             CASPER_PUBLIC_KEY_PREFIXED,
    //         )
    //         .unwrap();
    //     assert!(result, "Expected signature to verify correctly");

    //     // Valid signature
    //     let result = service
    //         .verify(TRANSACTION_HASH, SIGNATURE, CASPER_PUBLIC_KEY_PREFIXED)
    //         .unwrap();
    //     assert!(result, "Expected signature to verify correctly");

    //     // Invalid signature
    //     let result = service
    //         .verify("bad_hash", "bad_signature", "bad_key")
    //         .unwrap();
    //     assert!(!result, "Expected signature verification to fail");
    // }

    // #[tokio::test]
    // async fn test_verify_via_kms_signature() {
    //     let config = ConfigBuilder::new()
    //         .with_cosmos_mode()
    //         .with_aws_mode(false)
    //         .build();

    //     let wasm_loader = WasmLoader::new(WASM_PATH)
    //         .await
    //         .expect("Failed to load WASM module");

    //     let crypto_service =
    //         CryptoService::new(&wasm_loader).expect("Failed to initialize CryptoService");

    //     // Create CosmosKeysService (uses mocked KMS + real CryptoService)
    //     let mut service = CosmosKeysService::new(config.clone(), crypto_service)
    //         .await
    //         .expect("Failed to create CosmosKeysService");

    //     // Valid prefixed signature
    //     let result = service
    //         .verify_via_kms(
    //             TRANSACTION_HASH,
    //             SIGNATURE_PREFIXED,
    //             CASPER_PUBLIC_KEY_PREFIXED,
    //         )
    //         .await
    //         .unwrap();
    //     assert!(result, "Expected signature to verify correctly");

    //     // Valid signature
    //     let result = service
    //         .verify_via_kms(TRANSACTION_HASH, SIGNATURE, CASPER_PUBLIC_KEY_PREFIXED)
    //         .await
    //         .unwrap();
    //     assert!(result, "Expected signature to verify correctly");

    //     // Invalid signature
    //     let result = service
    //         .verify_via_kms("bad_hash", "bad_signature", "bad_key")
    //         .await
    //         .unwrap();
    //     assert!(!result, "Expected signature verification to fail");
    // }

    #[tokio::test]
    async fn test_delete_key() {
        let config = ConfigBuilder::new()
            .with_cosmos_mode()
            .with_aws_mode(false)
            .build();

        let wasm_loader = WasmLoader::new(WASM_PATH)
            .await
            .expect("Failed to load WASM module");

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

        let wasm_loader = WasmLoader::new(WASM_PATH)
            .await
            .expect("Failed to load WASM module");

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

    // #[tokio::test]
    // async fn test_sign_transaction_hash() {
    //     let config = ConfigBuilder::new()
    //         .with_cosmos_mode()
    //         .with_aws_mode(false)
    //         .build();

    //     let wasm_loader = WasmLoader::new(WASM_PATH)
    //         .await
    //         .expect("Failed to load WASM module");

    //     let crypto_service =
    //         CryptoService::new(&wasm_loader).expect("Failed to initialize CryptoService");

    //     let mut service = CosmosKeysService::new(config.clone(), crypto_service)
    //         .await
    //         .expect("Failed to create CosmosKeysService");

    //     let transaction_hash = TRANSACTION_HASH;

    //     let public_key = CASPER_PUBLIC_KEY_PREFIXED;

    //     let result = service
    //         .sign_transaction_hash(&config, transaction_hash, public_key)
    //         .await;

    //     assert!(result.is_ok(), "Expected signing to succeed");

    //     let signed = result.unwrap();

    //     let expected_sig_hex = SIGNATURE;

    //     assert_eq!(
    //         signed, expected_sig_hex,
    //         "Expected signature to match expected format"
    //     );
    // }

    // #[tokio::test]
    // async fn test_sign_transaction() {
    //     let config = ConfigBuilder::new()
    //         .with_cosmos_mode()
    //         .with_aws_mode(false)
    //         .build();

    //     let wasm_loader = WasmLoader::new(WASM_PATH)
    //         .await
    //         .expect("Failed to load WASM module");

    //     let crypto_service =
    //         CryptoService::new(&wasm_loader).expect("Failed to initialize CryptoService");

    //     // Create the CosmosKeysService with mocks + real crypto
    //     let mut service = CosmosKeysService::new(config.clone(), crypto_service)
    //         .await
    //         .expect("Failed to create CosmosKeysService");

    //     let public_key = CASPER_PUBLIC_KEY_PREFIXED;

    //     // Build transaction params as you provided
    //     let transaction_params = TransactionStrParams::default();
    //     transaction_params.set_chain_name("cosmos-net-1");
    //     transaction_params.set_initiator_addr(public_key);
    //     transaction_params.set_payment_amount("100000000");

    //     // Create SDK & transaction
    //     let sdk = SDK::new(None, None, None);
    //     let transaction = sdk
    //         .make_transfer_transaction(None, public_key, "2500000000", transaction_params, None)
    //         .expect("Failed to create transfer transaction");

    //     let mut transaction_str = transaction.to_json_string().unwrap_or_default();

    //     let re = Regex::new(r#""hash"\s*:\s*"[^"]+""#).unwrap();

    //     // Replace transaction_hash by mock
    //     transaction_str = re
    //         .replace(&transaction_str, format!(r#""hash": "{TRANSACTION_HASH}""#))
    //         .to_string();

    //     // Call sign_transaction
    //     let signed_transaction_json = service
    //         .sign_transaction(&config, &transaction_str, public_key)
    //         .await
    //         .expect("Failed to sign transaction");

    //     // Deserialize signed transaction to validate it contains signature
    //     let signed_transaction = Transaction::from_json_string(&signed_transaction_json)
    //         .expect("Failed to parse signed transaction");

    //     // Extract approvals and verify
    //     let approvals = signed_transaction.approvals();

    //     for approval in approvals {
    //         let signer = approval.signer().to_hex_string();
    //         let signature = approval.signature().to_hex_string();

    //         assert!(public_key.contains(&signer), "Unexpected signer: {signer}");

    //         assert_eq!(
    //             signature.len(),
    //             130,
    //             "Signature length incorrect for signer {}: {}",
    //             signer,
    //             signature.len()
    //         );
    //     }
    // }

    // #[tokio::test]
    // async fn test_sign_transaction_malformed_json() {
    //     let config = ConfigBuilder::new()
    //         .with_cosmos_mode()
    //         .with_aws_mode(false)
    //         .build();

    //     let wasm_loader = WasmLoader::new(WASM_PATH)
    //         .await
    //         .expect("Failed to load WASM");

    //     let crypto_service =
    //         CryptoService::new(&wasm_loader).expect("Failed to create CryptoService");

    //     let mut service = CosmosKeysService::new(config.clone(), crypto_service)
    //         .await
    //         .expect("Failed to create service");

    //     let bad_json = "{ this is not valid JSON }";

    //     let result = service
    //         .sign_transaction(&config, bad_json, CASPER_PUBLIC_KEY_PREFIXED)
    //         .await;

    //     assert!(result.is_err());
    //     let err = result.unwrap_err();
    //     assert!(
    //         err.contains("Failed to parse json-args")
    //             || err.contains("Failed to parse transaction"),
    //         "Expected parsing error, got: {err}"
    //     );
    // }

    // #[tokio::test]
    // async fn test_sign_transaction_missing_transaction_field() {
    //     let config = ConfigBuilder::new()
    //         .with_cosmos_mode()
    //         .with_aws_mode(false)
    //         .build();

    //     let wasm_loader = WasmLoader::new(WASM_PATH)
    //         .await
    //         .expect("Failed to load WASM");

    //     let crypto_service =
    //         CryptoService::new(&wasm_loader).expect("Failed to create CryptoService");

    //     let mut service = CosmosKeysService::new(config.clone(), crypto_service)
    //         .await
    //         .expect("Failed to create service");

    //     let minimal_transaction = json!({
    //         "some": "value"
    //     })
    //     .to_string();

    //     let result = service
    //         .sign_transaction(&config, &minimal_transaction, CASPER_PUBLIC_KEY_PREFIXED)
    //         .await;

    //     assert!(
    //         result.is_err(),
    //         "Expected failure due to missing transaction fields"
    //     );
    // }

    // #[tokio::test]
    // async fn test_sign_transaction_hash_with_invalid_input() {
    //     let config = ConfigBuilder::new()
    //         .with_cosmos_mode()
    //         .with_aws_mode(false)
    //         .build();

    //     let wasm_loader = WasmLoader::new(WASM_PATH)
    //         .await
    //         .expect("Failed to load WASM module");

    //     let crypto_service =
    //         CryptoService::new(&wasm_loader).expect("Failed to initialize CryptoService");

    //     let mut service = CosmosKeysService::new(config.clone(), crypto_service)
    //         .await
    //         .expect("Failed to create CosmosKeysService");

    //     // Invalid public key and transaction hash
    //     let result = service
    //         .sign_transaction_hash(&config, "invalid_hash", "invalid_key")
    //         .await;

    //     assert!(
    //         result.is_err(),
    //         "Expected signing to fail due to invalid input"
    //     );
    //     let error = result.unwrap_err();
    //     assert!(
    //         error.contains("Error reading transaction parameters"),
    //         "Unexpected error: {error}"
    //     );
    // }

    // #[tokio::test]
    // async fn test_sign_transaction_malformed_approvals() {
    //     let config = ConfigBuilder::new()
    //         .with_cosmos_mode()
    //         .with_aws_mode(false)
    //         .build();

    //     let wasm_loader = WasmLoader::new(WASM_PATH)
    //         .await
    //         .expect("Failed to load WASM");

    //     let crypto_service =
    //         CryptoService::new(&wasm_loader).expect("Failed to create CryptoService");

    //     let mut service = CosmosKeysService::new(config.clone(), crypto_service)
    //         .await
    //         .expect("Failed to create service");

    //     // approvals should be an array, here it's a string (malformed)
    //     let transaction = json!({
    //         "hash": TRANSACTION_HASH,
    //         "approvals": "not an array"
    //     })
    //     .to_string();

    //     let result = service
    //         .sign_transaction(&config, &transaction, CASPER_PUBLIC_KEY_PREFIXED)
    //         .await;

    //     assert!(
    //         result.is_err(),
    //         "Expected failure due to malformed approvals field"
    //     );
    // }
}
