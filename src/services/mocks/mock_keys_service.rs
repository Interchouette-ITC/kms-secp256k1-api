use crate::{
    config::Config,
    services::{crypto_service::CryptoService, keys_service::KeyEntry},
};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use k256::PublicKey;
use k256::pkcs8::EncodePublicKey;
use std::{collections::HashMap, sync::Arc};
use tokio::{
    sync::Mutex,
    time::{Duration, sleep},
};
use tracing::error;

#[derive(Debug, Clone)]
pub struct KeyPair {
    pub private_key: String,
    pub public_key: String,
    pub address: String,
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
    #[must_use]
    pub fn new(_config: Config, crypto_service: CryptoService) -> Self {
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
        key: &str,
    ) -> Result<bool, String> {
        self.crypto_service
            .verify(transaction_hash_hex, signature_hex, key)
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
        key: &str,
    ) -> Result<bool, String> {
        self.crypto_service
            .verify_eip155(transaction_hash_hex, signature_hex, key)
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
    /// [`verify`]: MockKeysService::verify
    pub async fn verify_via_kms(
        &mut self,
        transaction_hash_hex: &str,
        signature_hex: &str,
        key: &str,
    ) -> Result<bool, String> {
        sleep(Duration::from_millis(50)).await;
        self.verify(transaction_hash_hex, signature_hex, key)
    }

    /// Verifies an EIP-155 signature against the given transaction hash and key,
    /// introducing a small delay to simulate or throttle KMS behavior.
    ///
    /// This method is functionally equivalent to [`MockKeysService::verify_eip155`] but includes a
    /// `50ms` sleep before performing the verification. This is useful when simulating
    /// KMS latency or reducing load on dependent services in testing environments.
    ///
    /// # Parameters
    /// - `transaction_hash_hex`: The hex-encoded transaction hash that was signed.
    /// - `signature_hex`: The hex-encoded EIP-155 signature to verify.
    /// - `key`: A compressed secp256k1 public key or a KMS alias used to resolve the key.
    ///
    /// # Returns
    /// - `Ok(true)`: If the signature is valid.
    /// - `Ok(false)`: If the signature is invalid (cryptographic check fails).
    /// - `Err(String)`: If key resolution or verification fails due to internal errors or invalid input.
    ///
    /// # Errors
    /// Returns an error if the key is invalid, the signature is malformed,
    /// or the cryptographic operation fails internally.
    pub async fn verify_via_kms_eip155(
        &mut self,
        transaction_hash_hex: &str,
        signature_hex: &str,
        key: &str,
    ) -> Result<bool, String> {
        sleep(Duration::from_millis(50)).await;
        self.verify_eip155(transaction_hash_hex, signature_hex, key)
    }

    /// Deletes a public key from the internal key storage.
    ///
    /// Returns `true` if the key existed and was removed, `false` otherwise.
    pub async fn delete_key(&self, key: &str) -> bool {
        self.keys.lock().await.remove(key).is_some() // key alias used is key_pair address
    }

    /// Lists all stored keys with mock metadata.
    ///
    /// Returns a vector of `KeyEntry` representing all keys.
    pub async fn list_keys(&self) -> Vec<KeyEntry> {
        self.keys
            .lock()
            .await
            .values()
            .enumerate()
            .map(|(i, keypair)| {
                let public_key_base64 = hex::decode(&keypair.public_key)
                    .ok()
                    .and_then(|bytes| PublicKey::from_sec1_bytes(&bytes).ok())
                    .and_then(|pk| pk.to_public_key_der().ok())
                    .map(|der| STANDARD.encode(der))
                    .unwrap_or_default();

                KeyEntry {
                    address: keypair.address.clone().into(),
                    public_key_base64: public_key_base64.into(),
                    public_key: Some(keypair.public_key.clone()).into(),
                    key_id: format!("mock-key-{}", i + 1).into(),
                }
            })
            .collect()
    }

    /// Inserts a new key pair into the internal storage.
    ///
    /// Used to add keys for testing purposes.
    pub async fn insert_key(&self, key_pair: KeyPair) {
        let key = key_pair.address.clone(); // key alias used is key_pair address
        self.keys.lock().await.insert(key, key_pair);
    }
}
