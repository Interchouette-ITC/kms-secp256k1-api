use crate::config::Config;
use crate::constants::CASPER_SECP_PREFIX;
use crate::services::crypto_service::CryptoService;
use crate::services::keys_service::{KeysService, KeysServiceTrait};
use casper_rust_wasm_sdk::types::hash::transaction_hash::TransactionHash;
use casper_rust_wasm_sdk::types::public_key::PublicKey;
use casper_rust_wasm_sdk::types::transaction::Transaction;
use tracing::{error, info};

pub struct CasperKeysService {
    keys_service: KeysService,
}

impl CasperKeysService {
    pub async fn new(config: Config, crypto_service: CryptoService) -> Result<Self, String> {
        let keys_service = KeysService::new(config, crypto_service).await?;
        Ok(Self { keys_service })
    }
}

#[async_trait::async_trait]
impl KeysServiceTrait for CasperKeysService {
    /// Creates a new KMS key, derives its public key with an optional prefix keys_serviced on config,
    /// registers an alias for it, and returns the formatted public key.
    ///
    /// # Arguments
    ///
    /// * `config` - Reference to the config struct used to decide prefixing.
    ///
    /// # Errors
    ///
    /// Returns an error if key creation, public key conversion, or alias creation fails.
    async fn create_key(&mut self, config: &Config) -> Result<String, String> {
        // Determine prefix keys_serviced on config
        let prefix = if config.casper_mode {
            CASPER_SECP_PREFIX
        } else {
            ""
        };

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

        let mut public_key = self
            .keys_service
            .crypto_service
            .public_key(&public_key)
            .map_err(|e| {
                let msg = format!("public_key conversion failed: {e:?}");
                error!("{}", &msg);
                msg
            })?;

        // Prefix the public key if needed
        public_key = format!("{}{}", prefix, &public_key);

        if public_key.is_empty() {
            let msg = "No public key generated".to_string();
            error!("{}", &msg);
            return Err(msg);
        }

        info!("Public key retrieved: {}", public_key);

        // Create alias for the key
        self.keys_service
            .kms_client_service
            .create_alias(&key_id, &public_key)
            .await
            .map_err(|e| {
                let msg = format!("Error creating alias: {e:?}");
                error!("{}", &msg);
                msg
            })?;

        Ok(public_key)
    }

    /// Signs a transaction hash using the provided public key and configuration mode.
    ///
    /// Selects a prefix keys_serviced on the active mode (Casper or Ethereum),
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
        public_key: &str,
    ) -> Result<String, String> {
        if !config.casper_mode {
            return Err("Only Casper mode is supported".to_string());
        }
        info!("transaction_hash to sign: {}", transaction_hash);

        if let Err(e) = TransactionHash::new(transaction_hash) {
            info!(
                "Error reading parameters \npublic_key : {}\ntransaction_hash : {}",
                public_key, transaction_hash
            );
            error!("Validation error: {:?}", e);
            return Err("Error reading transaction parameters".to_string());
        }

        if let Err(e) = PublicKey::new(public_key) {
            info!(
                "Error reading parameters \npublic_key : {}\ntransaction_hash : {}",
                public_key, transaction_hash
            );
            error!("Validation error: {:?}", e);
            return Err("Error reading transaction parameters".to_string());
        }

        let signature = self
            .keys_service
            .sign(transaction_hash, public_key, Some(CASPER_SECP_PREFIX))
            .await?;

        // Now verify the signature immediately
        let verified = self.verify(transaction_hash, &signature, public_key)?;
        if !verified {
            return Err("Signature verification failed after signing".to_string());
        }

        Ok(signature)
    }

    /// Signs a serialized Casper transaction using the provided public key.
    ///
    /// Parses the input transaction string, verifies the transaction hash and public key,
    /// signs the hash, attaches the signature to the transaction, and returns the signed
    /// transaction as a JSON string.
    ///
    /// # Arguments
    ///
    /// * `config` - Reference to the application configuration. Only Casper mode is supported.
    /// * `transaction_str` - A JSON string representing the transaction to be signed.
    /// * `public_key` - The public key corresponding to the signing key.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The application is not in Casper mode.
    /// - The transaction JSON is invalid.
    /// - The transaction hash or public key is invalid.
    /// - The signing process fails.
    /// - The final transaction serialization fails.
    async fn sign_transaction(
        &mut self,
        config: &Config,
        transaction_str: &str,
        public_key: &str,
    ) -> Result<String, String> {
        if !config.casper_mode {
            return Err("Only Casper mode is supported".to_string());
        }

        let transaction: Transaction = Transaction::from_json_string(transaction_str)
            .map_err(|e| format!("Failed to parse transaction: {e}"))?;

        let transaction_hash_str = transaction.hash().to_string();

        TransactionHash::new(&transaction_hash_str).map_err(|e| {
            error!("Invalid transaction hash: {:?}", e);
            "Invalid transaction hash".to_string()
        })?;

        PublicKey::new(public_key).map_err(|e| {
            error!("Invalid public key: {:?}", e);
            "Invalid public key".to_string()
        })?;

        let signature = self
            .keys_service
            .sign(&transaction_hash_str, public_key, Some(CASPER_SECP_PREFIX))
            .await
            .map_err(|e| format!("Signing failed: {e}"))?;

        // Verify the signature immediately
        let verified = self.verify(&transaction_hash_str, &signature, public_key)?;
        if !verified {
            return Err("Signature verification failed after signing".to_string());
        }

        let signed_transaction = transaction.add_signature(public_key, &signature);

        signed_transaction
            .to_json_string()
            .map_err(|e| format!("Failed to serialize signed transaction: {e}"))
    }

    fn verify(
        &mut self,
        transaction_hash_hex: &str,
        signature_hex: &str,
        public_key: &str,
    ) -> Result<bool, String> {
        self.keys_service
            .verify(transaction_hash_hex, signature_hex, public_key)
    }

    async fn verify_via_kms(
        &mut self,
        transaction_hash_hex: &str,
        signature_hex: &str,
        public_key: &str,
    ) -> Result<bool, String> {
        self.keys_service
            .verify_via_kms(transaction_hash_hex, signature_hex, public_key)
            .await
    }

    async fn delete_key(&mut self, public_key: &str) -> Result<bool, String> {
        self.keys_service.delete_key(public_key).await
    }

    async fn list_keys(&mut self) -> Result<Vec<(String, String)>, String> {
        self.keys_service.list_keys().await
    }
}

#[cfg(test)]
mod tests {
    use casper_rust_wasm_sdk::{
        SDK, types::transaction_params::transaction_str_params::TransactionStrParams,
    };
    use regex::Regex;

    use super::*;
    use crate::{
        config::Config,
        constants::{
            CASPER_PUBLIC_KEY_PREFIXED, SIGNATURE, SIGNATURE_PREFIXED, TRANSACTION_HASH, WASM_PATH,
        },
        services::crypto_service::CryptoService,
        wasm_loader::WasmLoader,
    };

    #[tokio::test]
    async fn test_create_key() {
        let config = Config {
            casper_mode: true,
            aws_mode: false,
            ..Default::default()
        };

        let wasm_loader = WasmLoader::new(WASM_PATH)
            .await
            .expect("Failed to load WASM module");

        let crypto_service =
            CryptoService::new(&wasm_loader).expect("Failed to initialize CryptoService");

        // Create the service via the real constructor, it will use MockKmsClientService internally
        let mut service = CasperKeysService::new(config.clone(), crypto_service)
            .await
            .expect("Failed to create CasperKeysService");

        // Call create_key on the service under test
        let result = service.create_key(&config).await;

        // Assert the key is returned with the casper prefix
        assert!(result.is_ok(), "create_key failed: {result:?}");
        let public_key = result.unwrap();
        assert!(public_key.starts_with(CASPER_SECP_PREFIX));
        assert!(!public_key.is_empty());
    }

    #[tokio::test]
    async fn test_verify_signature() {
        let config = Config {
            casper_mode: true,
            aws_mode: false,
            ..Default::default()
        };

        let wasm_loader = WasmLoader::new(WASM_PATH)
            .await
            .expect("Failed to load WASM module");

        let crypto_service =
            CryptoService::new(&wasm_loader).expect("Failed to initialize CryptoService");

        // Create CasperKeysService (uses mocked KMS + real CryptoService)
        let mut service = CasperKeysService::new(config.clone(), crypto_service)
            .await
            .expect("Failed to create CasperKeysService");

        //  Valid prefixed signature
        let result = service
            .verify(
                TRANSACTION_HASH,
                SIGNATURE_PREFIXED,
                CASPER_PUBLIC_KEY_PREFIXED,
            )
            .unwrap();
        assert!(result, "Expected signature to verify correctly");

        // Valid signature
        let result = service
            .verify(TRANSACTION_HASH, SIGNATURE, CASPER_PUBLIC_KEY_PREFIXED)
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
        let config = Config {
            casper_mode: true,
            aws_mode: false,
            ..Default::default()
        };

        let wasm_loader = WasmLoader::new(WASM_PATH)
            .await
            .expect("Failed to load WASM module");

        let crypto_service =
            CryptoService::new(&wasm_loader).expect("Failed to initialize CryptoService");

        // Create CasperKeysService (uses mocked KMS + real CryptoService)
        let mut service = CasperKeysService::new(config.clone(), crypto_service)
            .await
            .expect("Failed to create CasperKeysService");

        // Valid prefixed signature
        let result = service
            .verify_via_kms(
                TRANSACTION_HASH,
                SIGNATURE_PREFIXED,
                CASPER_PUBLIC_KEY_PREFIXED,
            )
            .await
            .unwrap();
        assert!(result, "Expected signature to verify correctly");

        // Valid signature
        let result = service
            .verify_via_kms(TRANSACTION_HASH, SIGNATURE, CASPER_PUBLIC_KEY_PREFIXED)
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
        let config = Config {
            casper_mode: true,
            aws_mode: false,
            ..Default::default()
        };

        let wasm_loader = WasmLoader::new(WASM_PATH)
            .await
            .expect("Failed to load WASM module");

        let crypto_service =
            CryptoService::new(&wasm_loader).expect("Failed to initialize CryptoService");

        let mut service = CasperKeysService::new(config.clone(), crypto_service)
            .await
            .expect("Failed to create CasperKeysService");

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
        let config = Config {
            casper_mode: true,
            aws_mode: false,
            ..Default::default()
        };

        let wasm_loader = WasmLoader::new(WASM_PATH)
            .await
            .expect("Failed to load WASM module");

        let crypto_service =
            CryptoService::new(&wasm_loader).expect("Failed to initialize CryptoService");

        let mut service = CasperKeysService::new(config.clone(), crypto_service)
            .await
            .expect("Failed to create CasperKeysService");

        let result = service.list_keys().await;

        assert!(result.is_ok(), "Expected list_keys to succeed");

        let keys = result.unwrap();
        assert_eq!(keys.len(), 2);
        assert_eq!(
            keys[0],
            ("key_id_1".to_string(), "public_key_1".to_string())
        );
        assert_eq!(
            keys[1],
            ("key_id_2".to_string(), "public_key_2".to_string())
        );
    }

    #[tokio::test]
    async fn test_sign_transaction_hash() {
        let config = Config {
            casper_mode: true,
            aws_mode: false,
            ..Default::default()
        };

        let wasm_loader = WasmLoader::new(WASM_PATH)
            .await
            .expect("Failed to load WASM module");

        let crypto_service =
            CryptoService::new(&wasm_loader).expect("Failed to initialize CryptoService");

        let mut service = CasperKeysService::new(config.clone(), crypto_service)
            .await
            .expect("Failed to create CasperKeysService");

        let transaction_hash = TRANSACTION_HASH;

        let public_key = CASPER_PUBLIC_KEY_PREFIXED;

        let result = service
            .sign_transaction_hash(&config, transaction_hash, public_key)
            .await;

        assert!(result.is_ok(), "Expected signing to succeed");

        let signed = result.unwrap();

        let expected_sig_hex = SIGNATURE;

        let expected_prefixed = format!("{CASPER_SECP_PREFIX}{expected_sig_hex}");

        assert_eq!(
            signed, expected_prefixed,
            "Expected signature to match expected format"
        );
    }

    #[tokio::test]
    async fn test_sign_transaction() {
        let config = Config {
            casper_mode: true,
            aws_mode: false,
            ..Default::default()
        };

        let wasm_loader = WasmLoader::new(WASM_PATH)
            .await
            .expect("Failed to load WASM module");

        let crypto_service =
            CryptoService::new(&wasm_loader).expect("Failed to initialize CryptoService");

        // Create the CasperKeysService with mocks + real crypto
        let mut service = CasperKeysService::new(config.clone(), crypto_service)
            .await
            .expect("Failed to create CasperKeysService");

        let public_key = CASPER_PUBLIC_KEY_PREFIXED;

        // Build transaction params as you provided
        let transaction_params = TransactionStrParams::default();
        transaction_params.set_chain_name("mock");
        transaction_params.set_initiator_addr(public_key);
        transaction_params.set_payment_amount("100000000");

        // Create SDK & transaction
        let sdk = SDK::new(None, None, None);
        let transaction = sdk
            .make_transfer_transaction(None, public_key, "2500000000", transaction_params, None)
            .expect("Failed to create transfer transaction");

        let mut transaction_str = transaction.to_json_string().unwrap_or_default();

        let re = Regex::new(r#""hash"\s*:\s*"[^"]+""#).unwrap();

        // Replace transaction_hash by mock
        transaction_str = re
            .replace(&transaction_str, format!(r#""hash": "{TRANSACTION_HASH}""#))
            .to_string();

        // Call sign_transaction
        let signed_transaction_json = service
            .sign_transaction(&config, &transaction_str, public_key)
            .await
            .expect("Failed to sign transaction");

        // Deserialize signed transaction to validate it contains signature
        let signed_transaction = Transaction::from_json_string(&signed_transaction_json)
            .expect("Failed to parse signed transaction");

        // Extract approvals and verify
        let approvals = signed_transaction.approvals();

        for approval in approvals {
            let signer = approval.signer().to_hex_string();
            let signature = approval.signature().to_hex_string();

            assert!(public_key.contains(&signer), "Unexpected signer: {signer}");

            assert_eq!(
                signature.len(),
                130,
                "Signature length incorrect for signer {}: {}",
                signer,
                signature.len()
            );
        }
    }
}
