#[cfg(test)]
use crate::services::mocks::mock_kms_client_service::MockKmsClientService;
use crate::services::{
    aws_kms_client_service::AWSKmsClientService, crypto_service::CryptoService,
    kms_client_service::KmsClientService,
};
use crate::{config::Config, constants::SIGNATURE_RSV_LEN};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::error;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct KeyEntry {
    pub address: Arc<String>,
    pub public_key: Arc<Option<String>>,
    pub public_key_base64: Arc<String>,
    pub key_id: Arc<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct SigEntry {
    pub address: Arc<String>,
    pub public_key: Arc<String>,
    pub signature: Arc<String>,
}

#[async_trait::async_trait]
pub trait KeysServiceTrait: Send + Sync {
    /// Creates a new cryptographic key based on the provided configuration.
    ///
    /// # Errors
    /// Returns an error if key creation fails due to misconfiguration, internal cryptographic errors,
    /// or if the underlying storage backend is unavailable.
    async fn create_key(&mut self, config: &Config) -> crate::Result<KeyEntry>;

    /// Signs the given transaction hash using the specified public key.
    ///
    /// # Arguments
    /// - `transaction_hash`: A hex-encoded hash of the transaction data.
    /// - `key`: The key used to locate the corresponding private key for signing.
    ///
    /// # Errors
    /// Returns an error if the key is not found, if signing fails, or if the inputs are malformed.
    async fn sign_transaction_hash(
        &mut self,
        config: &Config,
        transaction_hash: &str,
        key: &str,
    ) -> crate::Result<SigEntry>;

    /// Signs the raw transaction data using the specified public key.
    ///
    /// # Arguments
    /// - `transaction_str`: The raw transaction string to be hashed and signed.
    /// - `key`: The public key used to determine the signing key.
    ///
    /// # Errors
    /// Returns an error if hashing, signing, or key retrieval fails.
    async fn sign_transaction(
        &mut self,
        config: &Config,
        transaction_str: &str,
        key: &str,
    ) -> crate::Result<String>;

    /// Verifies that the given signature is valid for the provided transaction hash and public key.
    ///
    /// # Arguments
    /// - `transaction_hash_hex`: A hex-encoded transaction hash.
    /// - `signature_hex`: A hex-encoded digital signature.
    /// - `key`: The key alias to verify against.
    ///
    /// # Errors
    /// Returns an error if the signature is invalid, the input format is incorrect,
    /// or if verification logic encounters a failure.
    async fn verify(
        &mut self,
        transaction_hash_hex: &str,
        signature_hex: &str,
        key: &str,
    ) -> crate::Result<bool>;

    /// Verifies a transaction hash using an external KMS (Key Management System).
    ///
    /// # Arguments
    /// - `transaction_hash_hex`: A hex-encoded hash of the transaction.
    /// - `signature_hex`: A hex-encoded signature to verify.
    /// - `key`: The key alias in the KMS.
    ///
    /// # Errors
    /// Returns an error if KMS access fails, if the inputs are malformed,
    /// or if verification returns an invalid result.
    async fn verify_via_kms(
        &mut self,
        transaction_hash_hex: &str,
        signature_hex: &str,
        key: &str,
    ) -> crate::Result<bool>;

    /// Deletes the cryptographic key associated with the given key.
    ///
    /// # Arguments
    /// - `key`: The key whose associated private key should be deleted.
    ///
    /// # Errors
    /// Returns an error if the key cannot be found or deletion fails due to internal issues or access control.
    async fn delete_key(&mut self, key: &str) -> crate::Result<bool>;

    /// Lists all available public keys and their metadata.
    ///
    /// # Errors
    /// Returns an error if key listing fails due to storage access problems or unexpected internal errors.
    async fn list_keys(&mut self) -> crate::Result<Vec<KeyEntry>>;
}

pub struct KeysService {
    pub kms_client_service: Arc<dyn KmsClientService>,
    pub crypto_service: CryptoService,
}

impl KeysService {
    /// Creates a new `KeyService` instance with the given config and crypto service.
    ///
    /// Initializes the appropriate KMS client (AWS or else) based on the config.
    ///
    /// # Errors
    ///
    /// Returns an error if the AWS KMS client fails to initialize.
    ///
    pub async fn new(config: Config, crypto_service: CryptoService) -> crate::Result<Self> {
        let kms_client_service: Arc<dyn KmsClientService> = if config.is_aws_mode() {
            let client = AWSKmsClientService::new(config.get_aws_config().clone())
                .await
                .map_err(|e| {
                    crate::KmsError::Msg(format!("Failed to initialize AWSKmsClientService: {e}"))
                })?;
            Arc::new(client)
        } else if config.is_testing_mode() {
            Self::get_mock_kms_client_service()
        } else {
            return Err(crate::KmsError::UnsupportedMode);
        };

        Ok(Self {
            kms_client_service,
            crypto_service,
        })
    }

    #[cfg(test)]
    fn get_mock_kms_client_service() -> Arc<dyn KmsClientService> {
        Arc::new(MockKmsClientService)
    }

    #[cfg(not(test))]
    fn get_mock_kms_client_service() -> Arc<dyn KmsClientService> {
        unimplemented!("Mock KMS Client is only available in tests")
    }

    /// Signs a given transaction hash using the specified key.
    ///
    /// This function delegates signing to the underlying KMS client service, then converts the
    /// resulting signature into the expected format. An optional prefix can be prepended to
    /// the final signature string.
    ///
    /// # Parameters
    /// - `transaction_hash_hex`: The transaction hash to sign, as a hex-encoded string.
    /// - `key`: The key or alias to use for signing.
    /// - `prefix`: An optional string slice to prepend to the signature (e.g., a format or type prefix).
    ///
    /// # Returns
    /// - `Ok(String)`: The formatted signature string, optionally prefixed.
    /// - `Err(String)`: An error message if signing or signature conversion fails.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The KMS client service fails to produce a signature for the given hash and key.
    /// - Conversion of the raw signature bytes into the desired format fails.
    pub async fn sign(
        &mut self,
        transaction_hash_hex: &str,
        key: &str,
        prefix: Option<&str>,
    ) -> crate::Result<String> {
        let signature = self
            .kms_client_service
            .sign(transaction_hash_hex, key)
            .await
            .map_err(|e| {
                let msg = format!("Failed to sign transaction with KMS: {e}");
                error!("{}", msg);
                crate::KmsError::Msg(msg)
            })?;

        let mut signature = self.crypto_service.convert(&signature).inspect_err(|e| {
            error!(error = %e, "Failed to convert signature");
        })?;

        if let Some(pref) = prefix {
            signature = format!("{pref}{signature}");
        }

        Ok(signature)
    }

    /// Verifies a signature against a transaction hash and public key.
    ///
    /// # Arguments
    ///
    /// * `transaction_hash_hex` - The hex-encoded transaction hash to verify.
    /// * `signature_hex` - The hex-encoded signature to verify.
    /// * `public_key` - The public key to use for verification.
    ///
    /// # Errors
    ///
    /// Returns an error string if the verification process fails.
    ///
    /// # Returns
    ///
    /// `Ok(true)` if the signature is valid, `Ok(false)` if invalid.
    pub fn verify(
        &mut self,
        transaction_hash_hex: &str,
        signature_hex: &str,
        public_key: &str,
    ) -> crate::Result<bool> {
        self.crypto_service
            .verify(transaction_hash_hex, signature_hex, public_key)
            .inspect_err(|e| {
                error!(error = %e, "Signature verification failed");
            })
    }

    /// Verifies an Ethereum EIP-155 signature against a transaction hash and public key.
    ///
    /// # Arguments
    ///
    /// * `transaction_hash_hex` - The hex-encoded transaction hash to verify.
    /// * `signature_hex` - The hex-encoded signature to verify.
    /// * `public_key` - The public key to use for verification.
    ///
    /// # Errors
    ///
    /// Returns an error string if the verification process fails.
    ///
    /// # Returns
    ///
    /// `Ok(true)` if the signature is valid, `Ok(false)` if invalid.
    pub fn verify_eip155(
        &mut self,
        transaction_hash_hex: &str,
        signature_hex: &str,
        public_key: &str,
    ) -> crate::Result<bool> {
        self.crypto_service
            .verify_eip155(transaction_hash_hex, signature_hex, public_key)
            .inspect_err(|e| {
                error!(error = %e, "Signature verification failed");
            })
    }

    /// Verifies a signature against a transaction hash and public key using both
    /// local crypto verification and KMS verification.
    ///
    /// First, verifies the signature locally via the crypto service. If that succeeds,
    /// converts the signature to ASN.1 base64 format and verifies it via the KMS client.
    ///
    /// # Arguments
    ///
    /// * `transaction_hash_hex` - Hex-encoded transaction hash to verify.
    /// * `signature_hex` - Hex-encoded signature to verify.
    /// * `public_key` - Public key used for verification.
    ///
    /// # Errors
    ///
    /// Returns an error string if either local or KMS verification fails.
    ///
    /// # Returns
    ///
    /// `Ok(true)` if both verifications pass, `Ok(false)` if local verification fails.
    pub async fn verify_via_kms(
        &mut self,
        transaction_hash_hex: &str,
        signature_hex: &str,
        public_key: &str,
    ) -> crate::Result<bool> {
        self.verify_via_kms_internal(transaction_hash_hex, signature_hex, public_key, false)
            .await
    }

    /// Verifies an EIP-155 signature and then confirms it via KMS.
    ///
    /// This method first performs a cryptographic verification of the signature according to EIP-155.
    /// If that succeeds, it proceeds to verify the signature via the KMS service.
    ///
    /// # Parameters
    /// - `transaction_hash_hex`: The transaction hash to verify, as a hex-encoded string.
    /// - `signature_hex`: The signature to verify, as a hex-encoded string.
    /// - `public_key`: The public key corresponding to the signer, as a hex string.
    ///
    /// # Returns
    /// - `Ok(true)` if both cryptographic and KMS verifications succeed.
    /// - `Ok(false)` if the cryptographic verification fails.
    /// - `Err(String)` if any error occurs during verification.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The inputs cannot be parsed or decoded correctly.
    /// - The cryptographic verification process encounters an unexpected error.
    /// - The KMS verification call fails or returns an error.
    pub async fn verify_via_kms_eip155(
        &mut self,
        transaction_hash_hex: &str,
        signature_hex: &str,
        public_key: &str,
    ) -> crate::Result<bool> {
        self.verify_via_kms_internal(transaction_hash_hex, signature_hex, public_key, true)
            .await
    }

    async fn verify_via_kms_internal(
        &mut self,
        transaction_hash_hex: &str,
        mut signature_hex: &str,
        public_key: &str,
        use_eip155: bool,
    ) -> crate::Result<bool> {
        let is_verified = if use_eip155 {
            let verify_eip155 = self
                .verify_eip155(transaction_hash_hex, signature_hex, public_key)
                .inspect_err(|e| {
                    error!(error = %e, "Signature eip155 verification failed");
                })?;

            // Trim the last v byte for KMS verification if signature length matches
            if signature_hex.len() == SIGNATURE_RSV_LEN {
                signature_hex = &signature_hex[..signature_hex.len() - 2];
            }

            verify_eip155
        } else {
            self.verify(transaction_hash_hex, signature_hex, public_key)
                .inspect_err(|e| {
                    error!(error = %e, "Signature verification failed");
                })?
        };

        if !is_verified {
            return Ok(false);
        }

        let signature = self
            .crypto_service
            .unconvert(signature_hex)
            .inspect_err(|e| {
                error!(error = %e, "Signature conversion failed");
            })?;

        let alias = if use_eip155 {
            self.crypto_service
                .address_eth(public_key)
                .inspect_err(|e| {
                    error!(error = %e, "Failed to convert public key to address");
                })?
        } else {
            public_key.to_string()
        };

        self.kms_client_service
            .verify(transaction_hash_hex, &signature, &alias)
            .await
            .map_err(|e| {
                let msg = format!("Failed to verify signature with KMS: {e}");
                error!("{}", msg);
                crate::KmsError::Msg(msg)
            })
    }

    /// Creates a KMS key and derives `(key_id, public_key_base64, public_key)`.
    ///
    /// Caller resolves the chain-specific address from `public_key`, then registers
    /// the alias via [`Self::create_alias`].
    ///
    /// # Errors
    ///
    /// Returns an error if key creation or public key conversion fails.
    pub(crate) async fn create_kms_key(&mut self) -> crate::Result<(String, String, String)> {
        let (key_id, public_key_base64) =
            self.kms_client_service.create_key().await.map_err(|e| {
                let msg = format!("Failed to create_key in KmsClientService: {e}");
                error!("{}", &msg);
                crate::KmsError::Msg(msg)
            })?;

        let public_key = self
            .crypto_service
            .public_key(&public_key_base64)
            .inspect_err(|e| {
                error!(error = %e, "public_key conversion failed");
            })?;

        if public_key.is_empty() {
            error!("No public key generated");
            return Err(crate::KmsError::EmptyPublicKey);
        }

        Ok((key_id, public_key_base64, public_key))
    }

    /// Registers a KMS alias for an existing key id.
    ///
    /// # Errors
    ///
    /// Returns an error if alias creation fails.
    pub(crate) async fn create_alias(&self, key_id: &str, alias: &str) -> crate::Result<()> {
        self.kms_client_service
            .create_alias(key_id, alias)
            .await
            .map_err(|e| {
                let msg = format!("Error creating alias: {e:?}");
                error!("{}", &msg);
                crate::KmsError::Msg(msg)
            })
    }

    /// Deletes a key identified by the given key using the KMS client.
    ///
    /// # Arguments
    ///
    /// * `key` - The key identifying the key to delete.
    ///
    /// # Errors
    ///
    /// Returns an error string if the deletion fails.
    ///
    /// # Returns
    ///
    /// `Ok(true)` if the key was successfully deleted.
    pub async fn delete_key(&mut self, key: &str) -> crate::Result<bool> {
        self.kms_client_service.delete_key(key).await.map_err(|e| {
            let msg = format!("Key deletion failed: {e}");
            error!("{}", msg);
            crate::KmsError::Msg(msg)
        })
    }

    /// Retrieves a list of keys from the KMS client.
    ///
    /// # Errors
    ///
    /// Returns an error string if the key listing fails.
    ///
    /// # Returns
    ///
    /// `Ok` with a vector of `KeyEntry`
    pub async fn list_keys(&mut self) -> crate::Result<Vec<KeyEntry>> {
        let entries = self.kms_client_service.list_keys().await.map_err(|e| {
            let msg = format!("Listing keys failed: {e}");
            error!("{}", msg);
            crate::KmsError::Msg(msg)
        })?;

        let mut result = Vec::with_capacity(entries.len());

        for mut entry in entries {
            if entry.public_key.is_none() {
                let public_key_base64 = entry.public_key_base64.to_string();

                let public_key = self
                    .crypto_service
                    .public_key(&public_key_base64)
                    .inspect_err(|e| {
                        error!(error = %e, "public_key conversion failed");
                    })?;

                entry.public_key = Some(public_key).into();
            }
            result.push(entry);
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::ConfigBuilder,
        constants::{
            CASPER_PUBLIC_KEY_PREFIXED, CASPER_SECP_PREFIX, ETH_PUBLIC_KEY, ETH_SIGNATURE,
            ETH_TRANSACTION_HASH, SIGNATURE_RS_LEN, TRANSACTION_HASH, WASM_PATH,
        },
        services::crypto_service::CryptoService,
        wasm_loader::WasmLoader,
    };
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD;

    #[tokio::test]
    async fn test_sign_successful() {
        let config = ConfigBuilder::new().with_casper_mode().build();

        let wasm_loader = WasmLoader::new(WASM_PATH)
            .await
            .expect("Failed to load WASM module");

        let crypto_service =
            CryptoService::new(&wasm_loader).expect("Failed to initialize CryptoService");

        // Create CasperKeysService (uses mocked KMS + real CryptoService)
        let mut service = KeysService::new(config.clone(), crypto_service)
            .await
            .expect("Failed to create KeysService");

        let public_key = CASPER_PUBLIC_KEY_PREFIXED;

        // Call the sign method with prefix
        let result = service
            .sign(TRANSACTION_HASH, public_key, Some(CASPER_SECP_PREFIX))
            .await;
        assert!(result.is_ok(), "sign failed: {result:?}");
        let signature = result.unwrap();
        assert!(
            signature.starts_with(CASPER_SECP_PREFIX),
            "Signature missing prefix"
        );
        assert_eq!(
            signature.len(),
            CASPER_SECP_PREFIX.len() + SIGNATURE_RS_LEN,
            "Signature length invalid"
        );

        // Call the sign method without PREFIX
        let result = service.sign(TRANSACTION_HASH, public_key, None).await;
        assert!(result.is_ok(), "sign failed: {result:?}");
        let signature = result.unwrap();

        assert_eq!(
            signature.len(),
            SIGNATURE_RS_LEN,
            "Signature length invalid"
        );
    }

    #[tokio::test]
    async fn test_verify_successful() {
        let config = ConfigBuilder::new().with_casper_mode().build();

        let wasm_loader = WasmLoader::new(WASM_PATH)
            .await
            .expect("Failed to load WASM module");

        let crypto_service =
            CryptoService::new(&wasm_loader).expect("Failed to initialize CryptoService");

        let mut service = KeysService::new(config.clone(), crypto_service)
            .await
            .expect("Failed to create KeysService");

        let public_key = CASPER_PUBLIC_KEY_PREFIXED;

        // First, sign the transaction
        let signature = service
            .sign(TRANSACTION_HASH, public_key, None)
            .await
            .expect("Signing failed");

        // Then, verify the signature
        let result = service.verify(TRANSACTION_HASH, &signature, public_key);

        assert!(result.is_ok(), "verify failed: {result:?}");
        assert!(result.unwrap(), "signature verification returned false");
    }

    #[tokio::test]
    async fn test_verify_eip155_successful() {
        let config = ConfigBuilder::new().with_ethereum_mode().build();

        let wasm_loader = WasmLoader::new(WASM_PATH)
            .await
            .expect("Failed to load WASM module");

        let crypto_service =
            CryptoService::new(&wasm_loader).expect("Failed to initialize CryptoService");

        let mut service = KeysService::new(config.clone(), crypto_service)
            .await
            .expect("Failed to create KeysService");

        let transaction_hash = ETH_TRANSACTION_HASH;
        let public_key = ETH_PUBLIC_KEY;
        let signature = ETH_SIGNATURE;

        // Verify the known-good Ethereum signature
        let result = service.verify_eip155(transaction_hash, signature, public_key);

        assert!(result.is_ok(), "verify_eip155 failed: {result:?}");
        assert!(
            result.unwrap(),
            "EIP-155 signature verification returned false"
        );
    }

    #[tokio::test]
    async fn test_verify_via_kms_successful() {
        let config = ConfigBuilder::new().with_casper_mode().build();

        let wasm_loader = WasmLoader::new(WASM_PATH)
            .await
            .expect("Failed to load WASM module");

        let crypto_service =
            CryptoService::new(&wasm_loader).expect("Failed to initialize CryptoService");

        let mut service = KeysService::new(config, crypto_service)
            .await
            .expect("Failed to create KeysService");

        // Sign using Casper keys
        let signature = service
            .sign(TRANSACTION_HASH, CASPER_PUBLIC_KEY_PREFIXED, None)
            .await
            .expect("Signing failed");

        // Verify via KMS
        let result = service
            .verify_via_kms(TRANSACTION_HASH, &signature, CASPER_PUBLIC_KEY_PREFIXED)
            .await;

        assert!(result.is_ok(), "verify_via_kms failed: {result:?}");
        assert!(result.unwrap(), "KMS verification returned false");
    }

    #[tokio::test]
    async fn test_verify_via_kms_eip155_successful() {
        let config = ConfigBuilder::new().with_ethereum_mode().build();

        let wasm_loader = WasmLoader::new(WASM_PATH)
            .await
            .expect("Failed to load WASM module");

        let crypto_service =
            CryptoService::new(&wasm_loader).expect("Failed to initialize CryptoService");

        let mut service = KeysService::new(config, crypto_service)
            .await
            .expect("Failed to create KeysService");

        // ETH_SIGNATURE is assumed valid, from constants
        let result = service
            .verify_via_kms_eip155(ETH_TRANSACTION_HASH, ETH_SIGNATURE, ETH_PUBLIC_KEY)
            .await;

        assert!(result.is_ok(), "verify_via_kms_eip155 failed: {result:?}");
        assert!(result.unwrap(), "EIP-155 + KMS verification returned false");
    }

    #[tokio::test]
    async fn test_delete_key_successful() {
        let config = ConfigBuilder::new().build();

        let wasm_loader = WasmLoader::new(WASM_PATH)
            .await
            .expect("Failed to load WASM module");

        let crypto_service =
            CryptoService::new(&wasm_loader).expect("Failed to initialize CryptoService");

        let mut service = KeysService::new(config, crypto_service)
            .await
            .expect("Failed to create KeysService");

        // Key that matches mocked successful condition
        let result = service.delete_key("known_public_key_xyz").await;

        assert!(result.is_ok(), "delete_key failed unexpectedly: {result:?}");
        assert!(result.unwrap(), "Expected key deletion to succeed");
    }

    #[tokio::test]
    async fn test_delete_key_not_found() {
        let config = ConfigBuilder::new().build();

        let wasm_loader = WasmLoader::new(WASM_PATH)
            .await
            .expect("Failed to load WASM module");

        let crypto_service =
            CryptoService::new(&wasm_loader).expect("Failed to initialize CryptoService");

        let mut service = KeysService::new(config, crypto_service)
            .await
            .expect("Failed to create KeysService");

        // Key that doesn't match mock condition
        let result = service.delete_key("some_other_key").await;

        assert!(result.is_ok(), "delete_key failed unexpectedly: {result:?}");
        assert!(
            !result.unwrap(),
            "Expected deletion to return false for unknown key"
        );
    }

    #[tokio::test]
    async fn test_list_keys_successful() {
        let config = ConfigBuilder::new().build();
        let wasm_loader = WasmLoader::new(WASM_PATH)
            .await
            .expect("Failed to load WASM module");

        let crypto_service =
            CryptoService::new(&wasm_loader).expect("Failed to initialize CryptoService");

        let mut service = KeysService::new(config, crypto_service)
            .await
            .expect("Failed to create KeysService");

        let result = service.list_keys().await;

        assert!(result.is_ok(), "list_keys failed: {result:?}");

        let keys = result.unwrap();
        assert_eq!(keys.len(), 2, "Expected two keys from mocked KMS");
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
}
