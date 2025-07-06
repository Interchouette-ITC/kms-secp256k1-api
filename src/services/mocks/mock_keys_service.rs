use crate::{config::Config, services::crypto_service::CryptoService};
use std::{collections::HashMap, sync::Arc};
use tokio::{
    sync::Mutex,
    time::{Duration, sleep},
};
use tracing::error;

#[derive(Debug, Clone)]
pub struct KeyPair {
    pub secret_key: String,
    pub public_key: String,
}

pub struct MockKeysService {
    pub keys: Arc<Mutex<HashMap<String, KeyPair>>>,
    pub crypto_service: CryptoService,
}

impl MockKeysService {
    /// Creates a new instance of `MockKeysService`.
    ///
    /// Initializes the internal key storage and creates an underlying `CasperKeysService`.
    ///
    /// # Panics
    ///
    /// This function will panic if creating the underlying `CasperKeysService` fails.
    pub async fn new(_config: Config, crypto_service: CryptoService) -> Self {
        Self {
            keys: Arc::new(Mutex::new(HashMap::new())),
            crypto_service,
        }
    }

    /// Verifies a signature for the given transaction hash and public key.
    ///
    /// Returns `Ok(true)` if the signature is valid, `Ok(false)` if invalid.
    ///
    /// # Errors
    ///
    /// Returns an error if the verification process encounters a failure, such as
    /// an internal error during signature verification. The error string provides
    /// details about the failure.
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

    /// Verifies an Ethereum EIP-155 signature for the given transaction hash and public key.
    ///
    /// Returns `Ok(true)` if the signature is valid, `Ok(false)` if invalid.
    ///
    /// # Errors
    ///
    /// Returns an error if the verification process encounters a failure, such as
    /// an internal error during signature verification. The error string provides
    /// details about the failure.
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

    /// Asynchronously verifies a signature via a simulated Key Management Service (KMS).
    ///
    /// Introduces an artificial delay to simulate latency.
    /// Returns the same results as the [`verify`] method.
    ///
    /// # Errors
    ///
    /// Returns an error if signature verification fails. The error string provides
    /// details about the verification failure.
    ///
    /// [`verify`]: crate::MockKeysService::verify
    pub async fn verify_via_kms(
        &mut self,
        transaction_hash_hex: &str,
        signature_hex: &str,
        public_key: &str,
    ) -> Result<bool, String> {
        sleep(Duration::from_millis(50)).await;
        self.verify(transaction_hash_hex, signature_hex, public_key)
    }

    pub async fn verify_via_kms_eip155(
        &mut self,
        transaction_hash_hex: &str,
        signature_hex: &str,
        public_key: &str,
    ) -> Result<bool, String> {
        sleep(Duration::from_millis(50)).await;
        self.verify_eip155(transaction_hash_hex, signature_hex, public_key)
    }

    /// Deletes a public key from the internal key storage.
    ///
    /// Returns `true` if the key existed and was removed, `false` otherwise.
    pub async fn delete_key(&self, public_key: &str) -> bool {
        self.keys.lock().await.remove(public_key).is_some()
    }

    /// Lists all stored keys with mock metadata.
    ///
    /// Returns a vector of tuples `(public_key, metadata)` representing all keys.
    pub async fn list_keys(&self) -> Vec<(String, String)> {
        self.keys
            .lock()
            .await
            .keys()
            .enumerate()
            .map(|(i, public_key)| (public_key.clone(), format!("mock-key-{}", i + 1)))
            .collect()
    }

    /// Inserts a new key pair into the internal storage.
    ///
    /// Used to add keys for testing purposes.
    pub async fn insert_key(&self, key_pair: KeyPair) {
        self.keys
            .lock()
            .await
            .insert(key_pair.public_key.clone(), key_pair);
    }
}
