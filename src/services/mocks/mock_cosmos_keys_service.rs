use crate::{
    config::Config,
    constants::{COSMOS_SECP_LEN, DEFAULT_COSMOS_UDENOM},
    services::{
        crypto_service::CryptoService,
        keys_service::{KeyEntry, KeysServiceTrait},
        mocks::mock_keys_service::{KeyPair, MockKeysService},
    },
};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use k256::{
    ecdsa::{Signature, SigningKey, signature::Signer, signature::SignerMut},
    elliptic_curve::rand_core::OsRng,
};
use serde_json::json;
use tracing::error;

pub struct MockCosmosKeysService {
    inner: MockKeysService,
    udenom: String,
}

#[async_trait::async_trait]
impl KeysServiceTrait for MockCosmosKeysService {
    async fn create_key(&mut self, _config: &Config) -> Result<KeyEntry, String> {
        let signing_key = SigningKey::random(&mut OsRng);
        let verifying_key = signing_key.verifying_key();
        let encoded_point = verifying_key.to_encoded_point(true);

        let secret_key_bytes = signing_key.to_bytes();
        let secret_key_hex = hex::encode(secret_key_bytes);
        let pubkey_bytes = encoded_point.as_bytes();
        let public_key = hex::encode(pubkey_bytes);
        let address = self.resolve_key(&public_key)?;

        {
            let key_pair = KeyPair {
                public_key: public_key.clone(),
                secret_key: secret_key_hex,
                address: address.clone(),
            };
            let mut keys = self.inner.keys.lock().await;
            keys.entry(address.clone()).or_insert(key_pair);
        }

        Ok(KeyEntry {
            public_key: Some(public_key.clone()).into(),
            address: address.clone().into(),
            public_key_base64: STANDARD.encode(public_key).into(),
            key_id: address.into(),
        })
    }

    /// Signs a given transaction hash using the specified key.
    ///
    /// Takes the transaction hash, key, and configuration.
    /// Returns the signature as a hexadecimal string if successful.
    /// Returns an error string if signing fails.
    async fn sign_transaction_hash(
        &mut self,
        _config: &Config,
        transaction_hash: &str,
        key: &str,
    ) -> Result<String, String> {
        let key = self.resolve_key(key)?;

        let key_pair = {
            let keys = self.inner.keys.lock().await;
            keys.get(&key)
                .ok_or_else(|| "Key not found".to_string())?
                .clone()
        };

        let hash_bytes = hex::decode(transaction_hash)
            .map_err(|e| format!("Invalid transaction hash hex: {e}"))?;

        if hash_bytes.len() != 32 {
            return Err("Transaction hash must be 32 bytes".to_string());
        }

        let secret_key_bytes = hex::decode(&key_pair.secret_key)
            .map_err(|e| format!("Failed to decode secret key: {e}"))?;

        let signing_key = SigningKey::from_slice(&secret_key_bytes)
            .map_err(|e| format!("Failed to create signing key: {e}"))?;

        let signature: Signature = signing_key.sign(&hash_bytes);
        let der = signature.to_der();
        let der_bytes = der.as_bytes();
        let base64_signature = STANDARD.encode(der_bytes);
        // let sig_hex_long = hex::encode(der_bytes);

        let signature_hex = self
            .inner
            .crypto_service
            .convert(&base64_signature)
            .map_err(|e| {
                let msg = format!("Failed to convert signature: {e}");
                error!("{}", msg);
                msg
            })?;

        // Verify signature
        let is_valid = self
            .verify(transaction_hash, &signature_hex, &key_pair.public_key)
            .await?;

        if !is_valid {
            return Err("Signature verification failed".to_string());
        }

        Ok(signature_hex)
    }

    /// Signs a transaction represented as a JSON string with the given public key.
    ///
    /// Returns the signed transaction as a JSON string on success.
    /// Returns an error string if parsing the transaction or signing fails.
    async fn sign_transaction(
        &mut self,
        _config: &Config,
        transaction_str: &str,
        public_key: &str,
    ) -> Result<String, String> {
        // Parse the transaction JSON (can be wrapped or plain)

        serde_json::to_string("")
            .map_err(|e| format!("Failed to serialize final signed transaction: {e}"))
    }

    /// Verifies an Cosmos EIP-155 signature for a given transaction hash and public key.
    ///
    /// Returns `Ok(true)` if the signature is valid, `Ok(false)` if invalid.
    /// Returns an error string for failures such as invalid formats.
    async fn verify(
        &mut self,
        transaction_hash_hex: &str,
        signature_hex: &str,
        key: &str,
    ) -> Result<bool, String> {
        let key = self.resolve_key(key)?;
        let key_pair = {
            let keys = self.inner.keys.lock().await;
            keys.get(&key)
                .ok_or_else(|| "Public key not found".to_string())?
                .clone()
        };
        self.inner
            .verify(transaction_hash_hex, signature_hex, &key_pair.public_key)
            .map_err(|e| {
                let msg = format!("Signature verification failed: {e}");
                error!("{}", msg);
                msg
            })
    }

    /// Verifies a signature via the Key Management Service (KMS).
    ///
    /// Asynchronous function that returns `Ok(true)` if verification succeeds,
    /// or an error string if it fails.
    async fn verify_via_kms(
        &mut self,
        transaction_hash_hex: &str,
        signature_hex: &str,
        key: &str,
    ) -> Result<bool, String> {
        let key = self.resolve_key(key)?;
        let key_pair = {
            let keys = self.inner.keys.lock().await;
            keys.get(&key)
                .ok_or_else(|| "Public key not found".to_string())?
                .clone()
        };
        self.inner
            .verify_via_kms(transaction_hash_hex, signature_hex, &key_pair.public_key)
            .await
    }

    /// Deletes a public key from storage.
    ///
    /// Returns `Ok(true)` if the key was deleted, `Ok(false)` if the key was not found.
    /// Returns an error string if deletion fails.
    async fn delete_key(&mut self, key: &str) -> Result<bool, String> {
        let key = self.resolve_key(key)?;
        Ok(self.inner.delete_key(&key).await)
    }

    /// Lists all stored keys along with their associated metadata.
    ///
    /// Returns a vector of KeyEntry` on success.
    /// Returns an error string on failure.
    async fn list_keys(&mut self) -> Result<Vec<KeyEntry>, String> {
        Ok(self.inner.list_keys().await)
    }
}

impl MockCosmosKeysService {
    /// Creates a new `MockCosmosKeysService` with an empty in-memory key store.
    ///
    /// # Arguments
    ///
    /// * `_config` - Unused configuration object, included for interface compatibility.
    /// * `crypto_service` - The crypto service to use for key operations.
    ///
    /// # Errors
    ///
    /// This function currently does not return an error, but it returns a `Result`
    /// to match a common interface and allow future fallibility.
    pub async fn new(config: Config, crypto_service: CryptoService) -> Result<Self, String> {
        let cosmos_udenom = config.get_cosmos_udenom();
        let udenom = match cosmos_udenom.as_str() {
            "" => DEFAULT_COSMOS_UDENOM,
            udenom => udenom,
        }
        .to_string();

        Ok(Self {
            inner: MockKeysService::new(config, crypto_service).await,
            udenom,
        })
    }

    /// Resolves a given key string to a Cosmos Bech32 address.
    ///
    /// This function checks whether the input `key` is a compressed secp256k1 public key
    /// (by comparing its length to the expected `COSMOS_SECP_LEN`). If so, it attempts to convert
    /// the public key to its corresponding Cosmos address using the underlying crypto service and the configured `udenom`.
    /// Otherwise, it assumes the key is already a valid address and returns it unchanged.
    ///
    /// # Parameters
    /// - `key`: A string that is either a Cosmos Bech32 address or a compressed public key (hex-encoded, 33 bytes).
    ///
    /// # Returns
    /// - `Ok(String)`: The resolved Cosmos address as a Bech32-encoded string.
    /// - `Err(String)`: An error message if the conversion from public key to address fails.
    ///
    /// # Errors
    /// - Returns an error if the input is assumed to be a public key and the address derivation fails.
    ///
    /// # Notes
    /// - The Bech32 address is generated using the provided `udenom` as prefix.
    /// - This function logs an error internally if conversion fails.
    fn resolve_key(&mut self, key: &str) -> Result<String, String> {
        if key.len() == COSMOS_SECP_LEN {
            self.inner
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
