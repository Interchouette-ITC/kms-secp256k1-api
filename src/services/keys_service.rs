#[cfg(test)]
use crate::services::mocks::mock_kms_client_service::MockKmsClientService;
use crate::services::{
    aws_kms_client_service::AWSKmsClientService, crypto_service::CryptoService,
    kms_client_service::KmsClientService,
};
use crate::{config::Config, constants::SIGNATURE_RSV_LEN};
use std::sync::Arc;
use tracing::{error, info};

#[async_trait::async_trait]
pub trait KeysServiceTrait: Send + Sync {
    /// Creates a new cryptographic key based on the provided configuration.
    ///
    /// # Errors
    /// Returns an error if key creation fails due to misconfiguration, internal cryptographic errors,
    /// or if the underlying storage backend is unavailable.
    async fn create_key(&mut self, config: &Config) -> Result<String, String>;

    /// Signs the given transaction hash using the specified public key.
    ///
    /// # Arguments
    /// - `transaction_hash`: A hex-encoded hash of the transaction data.
    /// - `public_key`: The key used to locate the corresponding private key for signing.
    ///
    /// # Errors
    /// Returns an error if the key is not found, if signing fails, or if the inputs are malformed.
    async fn sign_transaction_hash(
        &mut self,
        config: &Config,
        transaction_hash: &str,
        public_key: &str,
    ) -> Result<String, String>;

    /// Signs the raw transaction data using the specified public key.
    ///
    /// # Arguments
    /// - `transaction_str`: The raw transaction string to be hashed and signed.
    /// - `public_key`: The public key used to determine the signing key.
    ///
    /// # Errors
    /// Returns an error if hashing, signing, or key retrieval fails.
    async fn sign_transaction(
        &mut self,
        config: &Config,
        transaction_str: &str,
        public_key: &str,
    ) -> Result<String, String>;

    /// Verifies that the given signature is valid for the provided transaction hash and public key.
    ///
    /// # Arguments
    /// - `transaction_hash_hex`: A hex-encoded transaction hash.
    /// - `signature_hex`: A hex-encoded digital signature.
    /// - `public_key`: The public key to verify against.
    ///
    /// # Errors
    /// Returns an error if the signature is invalid, the input format is incorrect,
    /// or if verification logic encounters a failure.
    fn verify(
        &mut self,
        transaction_hash_hex: &str,
        signature_hex: &str,
        public_key: &str,
    ) -> Result<bool, String>;

    /// Verifies a transaction hash using an external KMS (Key Management System).
    ///
    /// # Arguments
    /// - `transaction_hash_hex`: A hex-encoded hash of the transaction.
    /// - `signature_hex`: A hex-encoded signature to verify.
    /// - `public_key`: The public key identifier in the KMS.
    ///
    /// # Errors
    /// Returns an error if KMS access fails, if the inputs are malformed,
    /// or if verification returns an invalid result.
    async fn verify_via_kms(
        &mut self,
        transaction_hash_hex: &str,
        signature_hex: &str,
        public_key: &str,
    ) -> Result<bool, String>;

    /// Deletes the cryptographic key associated with the given public key.
    ///
    /// # Arguments
    /// - `public_key`: The public key whose associated private key should be deleted.
    ///
    /// # Errors
    /// Returns an error if the key cannot be found or deletion fails due to internal issues or access control.
    async fn delete_key(&mut self, public_key: &str) -> Result<bool, String>;

    /// Lists all available public keys and their metadata.
    ///
    /// # Errors
    /// Returns an error if key listing fails due to storage access problems or unexpected internal errors.
    async fn list_keys(&mut self) -> Result<Vec<(String, String)>, String>;
}

pub struct KeysService {
    pub kms_client_service: Arc<dyn KmsClientService>,
    pub crypto_service: CryptoService,
    ethereum_mode: bool,
    eth_chain_id: u8,
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
    pub async fn new(config: Config, crypto_service: CryptoService) -> Result<Self, String> {
        let kms_client_service: Arc<dyn KmsClientService> = if config.aws_mode {
            let client = AWSKmsClientService::new(config.aws.clone())
                .await
                .map_err(|e| format!("Failed to initialize AWSKmsClientService: {e}"))?;
            Arc::new(client)
        } else if config.testing_mode {
            // TODO Implement other kms
            Self::get_mock_kms_client_service()
        } else {
            return Err("Unsupported KMS mode and not in testing".to_string());
        };

        Ok(Self {
            kms_client_service,
            crypto_service,
            ethereum_mode: config.ethereum_mode,
            eth_chain_id: config.eth_chain_id,
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

    pub async fn sign(
        &mut self,
        transaction_hash_hex: &str,
        public_key: &str,
        prefix: Option<&str>,
    ) -> Result<String, String> {
        let signature = self
            .kms_client_service
            .sign(transaction_hash_hex, public_key)
            .await
            .map_err(|e| {
                let msg = format!("Failed to sign transaction with KMS: {e}");
                error!("{}", msg);
                msg
            })?;

        let mut signature = self.crypto_service.convert(&signature).map_err(|e| {
            let msg = format!("Failed to convert signature: {e}");
            error!("{}", msg);
            msg
        })?;

        if self.ethereum_mode && signature.len() == 128 {
            info!("adding V");
            let v_hex: String = match self.crypto_service.recover_v(
                transaction_hash_hex,
                &signature,
                public_key,
                Some(self.eth_chain_id),
            ) {
                Ok(v) => v,
                Err(e) => {
                    error!("Failed to recover v: {}", e);
                    "".to_string()
                }
            };
            if !v_hex.is_empty() {
                signature.push_str(&v_hex);
            }
        }

        if let Some(pref) = prefix {
            info!("adding prefix");
            signature = format!("{pref}{signature}");
        }

        info!("Final signature: {}", signature);

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
    ) -> Result<bool, String> {
        self.crypto_service
            .verify(transaction_hash_hex, signature_hex, public_key)
            .map_err(|e| {
                let msg = format!("Signature verification failed: {e}");
                error!("{}", msg);
                msg
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
    ) -> Result<bool, String> {
        self.crypto_service
            .verify_eip155(transaction_hash_hex, signature_hex, public_key)
            .map_err(|e| {
                let msg = format!("Signature verification failed: {e}");
                error!("{}", msg);
                msg
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
    ) -> Result<bool, String> {
        self.verify_via_kms_internal(transaction_hash_hex, signature_hex, public_key, false)
            .await
    }

    /// Verifies an EIP-155 signature and then confirms it via KMS.
    ///
    /// Returns `Ok(true)` if both the cryptographic and KMS verifications succeed.
    /// Returns `Ok(false)` if the cryptographic verification fails.
    /// Returns `Err` if any part of the process encounters an error.
    pub async fn verify_via_kms_eip155(
        &mut self,
        transaction_hash_hex: &str,
        signature_hex: &str,
        public_key: &str,
    ) -> Result<bool, String> {
        self.verify_via_kms_internal(transaction_hash_hex, signature_hex, public_key, true)
            .await
    }

    async fn verify_via_kms_internal(
        &mut self,
        transaction_hash_hex: &str,
        mut signature_hex: &str,
        public_key: &str,
        use_eip155: bool,
    ) -> Result<bool, String> {
        let is_verified = if use_eip155 {
            let verify_eip155 = self
                .verify_eip155(transaction_hash_hex, signature_hex, public_key)
                .map_err(|e| {
                    let msg = format!("Signature eip155 verification failed: {e}");
                    error!("{}", msg);
                    msg
                })?;

            // Trim the last v byte for KMS verification if signature length matches
            if signature_hex.len() == SIGNATURE_RSV_LEN {
                signature_hex = &signature_hex[..signature_hex.len() - 2];
            }

            verify_eip155
        } else {
            self.verify(transaction_hash_hex, signature_hex, public_key)
                .map_err(|e| {
                    let msg = format!("Signature verification failed: {e}");
                    error!("{}", msg);
                    msg
                })?
        };

        if !is_verified {
            return Ok(false);
        }

        let signature = self.crypto_service.unconvert(signature_hex).map_err(|e| {
            let msg = format!("Signature conversion failed: {e}");
            error!("{}", msg);
            msg
        })?;

        self.kms_client_service
            .verify(transaction_hash_hex, &signature, public_key)
            .await
            .map_err(|e| {
                let msg = format!("Failed to verify signature with KMS: {e}");
                error!("{}", msg);
                msg
            })
    }

    /// Deletes a key identified by the given public key using the KMS client.
    ///
    /// # Arguments
    ///
    /// * `public_key` - The public key identifying the key to delete.
    ///
    /// # Errors
    ///
    /// Returns an error string if the deletion fails.
    ///
    /// # Returns
    ///
    /// `Ok(true)` if the key was successfully deleted.
    pub async fn delete_key(&mut self, public_key: &str) -> Result<bool, String> {
        self.kms_client_service
            .delete_key(public_key)
            .await
            .map_err(|e| {
                let msg = format!("SignatKey deletion failed: {e}");
                error!("{}", msg);
                msg
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
    /// `Ok` with a vector of tuples containing key identifiers and their aliases.
    pub async fn list_keys(&mut self) -> Result<Vec<(String, String)>, String> {
        self.kms_client_service.list_keys().await.map_err(|e| {
            let msg = format!("Listing keys failed: {e}");
            error!("{}", msg);
            msg
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::Config,
        constants::{
            CASPER_PUBLIC_KEY_PREFIXED, CASPER_SECP_PREFIX, ETH_PUBLIC_KEY, ETH_SIGNATURE,
            ETH_SIGNATURE_V, ETH_TRANSACTION_HASH, SIGNATURE_RSV_LEN, TRANSACTION_HASH, WASM_PATH,
        },
        services::crypto_service::CryptoService,
        wasm_loader::WasmLoader,
    };

    #[tokio::test]
    async fn test_sign_successful() {
        let config = Config {
            aws_mode: false,
            casper_mode: true,
            ..Default::default()
        };

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
            CASPER_SECP_PREFIX.len() + 128,
            "Signature length invalid"
        );

        // Call the sign method without PREFIX
        let result = service.sign(TRANSACTION_HASH, public_key, None).await;
        assert!(result.is_ok(), "sign failed: {result:?}");
        let signature = result.unwrap();

        assert_eq!(signature.len(), 128, "Signature length invalid");
    }

    #[tokio::test]
    async fn test_sign_eip155_successful() {
        let config = Config {
            aws_mode: false,
            ethereum_mode: true,
            ..Default::default()
        };

        let wasm_loader = WasmLoader::new(WASM_PATH)
            .await
            .expect("Failed to load WASM module");

        let crypto_service =
            CryptoService::new(&wasm_loader).expect("Failed to initialize CryptoService");

        // Create CasperKeysService (uses mocked KMS + real CryptoService)
        let mut service = KeysService::new(config.clone(), crypto_service)
            .await
            .expect("Failed to create KeysService");

        // Call the sign method without PREFIX
        let result = service
            .sign(ETH_TRANSACTION_HASH, ETH_PUBLIC_KEY, None)
            .await;
        assert!(result.is_ok(), "sign failed: {result:?}");
        let signature = result.unwrap();

        assert_eq!(
            signature.to_string(),
            format!("{ETH_SIGNATURE}{ETH_SIGNATURE_V}"),
            "Signature invalid"
        );
        assert_eq!(
            signature.len(),
            SIGNATURE_RSV_LEN,
            "Signature length invalid"
        );
    }
}
