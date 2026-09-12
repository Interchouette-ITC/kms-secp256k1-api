use crate::{
    config::Config,
    constants::ETH_SECP_LEN,
    services::{
        crypto_service::CryptoService,
        keys_service::{KeyEntry, KeysServiceTrait, SigEntry},
        mocks::mock_keys_service::{KeyPair, MockKeysService},
    },
};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use ethers_core::types::{H256, TransactionRequest, TxHash};
use ethers_signers::{LocalWallet, Signer};
use k256::{ecdsa::SigningKey, elliptic_curve::rand_core::OsRng};
use serde_json::json;
use std::str::FromStr;
use tracing::error;

pub struct MockEthereumKeysService {
    inner: MockKeysService,
}

#[async_trait::async_trait]
impl KeysServiceTrait for MockEthereumKeysService {
    /// Creates a new cryptographic key.
    ///
    /// Returns the public key encoded as a hexadecimal string on success.
    /// Returns an error string describing the issue on failure.
    async fn create_key(&mut self, _config: &Config) -> crate::Result<KeyEntry> {
        let signing_key = SigningKey::random(&mut OsRng);
        let verifying_key = signing_key.verifying_key();
        let encoded_point = verifying_key.to_encoded_point(true);

        let private_key_bytes = signing_key.to_bytes();
        let private_key = hex::encode(private_key_bytes);

        let pubkey_bytes = encoded_point.as_bytes();
        let public_key = hex::encode(pubkey_bytes);

        let address = self.resolve_key(&public_key)?;

        {
            let key_pair = KeyPair {
                public_key: public_key.clone(),
                private_key,
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
    ) -> crate::Result<SigEntry> {
        let key = self.resolve_key(key)?;

        let key_pair = {
            let keys = self.inner.keys.lock().await;
            keys.get(&key)
                .ok_or(crate::KmsError::PublicKeyNotFound)?
                .clone()
        };

        let tx_hash_bytes = hex::decode(transaction_hash)
            .map_err(|e| crate::KmsError::InvalidTxHashHex(e.to_string()))?;

        if tx_hash_bytes.len() != 32 {
            return Err(crate::KmsError::TxHashWrongLength);
        }

        let tx_hash = H256::from_slice(&tx_hash_bytes);

        let wallet: LocalWallet = key_pair.private_key.parse().map_err(|e| {
            crate::KmsError::Msg(format!("Failed to parse secret key into wallet: {e}"))
        })?;

        let signature = wallet
            .sign_hash(tx_hash)
            .map_err(|e| crate::KmsError::Msg(format!("Failed to sign hash: {e}")))?;

        // Verify signature
        let is_valid = self
            .verify(
                transaction_hash,
                &signature.to_string(),
                &key_pair.public_key,
            )
            .await?;

        if !is_valid {
            return Err(crate::KmsError::VerificationFailed);
        }

        let signature = hex::encode(signature.to_vec());
        Ok(SigEntry {
            address: key.into(),
            public_key: key_pair.public_key.into(),
            signature: signature.into(),
        })
    }

    /// Signs a transaction represented as a JSON string with the given public key.
    ///
    /// Returns the signed transaction as a JSON string on success.
    /// Returns an error string if parsing the transaction or signing fails.
    async fn sign_transaction(
        &mut self,
        _config: &Config,
        transaction_str: &str,
        key: &str,
    ) -> crate::Result<String> {
        // Parse the transaction JSON (can be wrapped or plain)
        let parsed: serde_json::Value = serde_json::from_str(transaction_str)
            .map_err(|e| crate::KmsError::ParseJson(e.to_string()))?;

        // Extract transaction and signatures array if wrapped
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
            _ => return Err(crate::KmsError::UnsupportedTxFormat),
        };

        // Deserialize transaction to struct for sighash
        let transaction: TransactionRequest = serde_json::from_value(transaction_value.clone())
            .map_err(|e| {
                crate::KmsError::ParseTransaction(format!("Failed to parse transaction: {e}"))
            })?;

        // Calculate sighash and validate
        let mut transaction_hash_str = format!("{:x}", transaction.sighash());
        transaction_hash_str = transaction_hash_str.trim_start_matches("0x").to_string();

        TxHash::from_str(&transaction_hash_str).map_err(|e| {
            error!("Invalid transaction hash: {:?}", e);
            "Invalid transaction hash".to_string()
        })?;

        // Get key pair and wallet
        let key = self.resolve_key(key)?;
        let key_pair = {
            let keys = self.inner.keys.lock().await;
            keys.get(&key)
                .ok_or(crate::KmsError::PublicKeyNotFound)?
                .clone()
        };

        let wallet: LocalWallet = key_pair.private_key.parse().map_err(|e| {
            crate::KmsError::Msg(format!("Failed to parse secret key into wallet: {e}"))
        })?;

        // Sign the transaction
        let signature = wallet
            .sign_transaction(&transaction.clone().into())
            .await
            .map_err(|e| crate::KmsError::Msg(format!("Failed to sign transaction: {e}")))?;

        let signature_hex = signature.to_string();

        // Verify signature
        let is_valid = self
            .verify(&transaction_hash_str, &signature_hex, &key_pair.public_key)
            .await?;
        if !is_valid {
            return Err(crate::KmsError::PostSignVerifyFailed);
        }

        // Append signature info
        signatures.push(json!({
            "address": key,
            "signer": &key_pair.public_key,
            "v": format!("{:x}", signature.v),
            "r": format!("{:x}", signature.r),
            "s": format!("{:x}", signature.s),
            "hash": transaction_hash_str,
            "signature": signature_hex
        }));

        // Return wrapped transaction + signatures
        let result = json!({
            "transaction": transaction_value,
            "signatures": signatures
        });

        serde_json::to_string(&result).map_err(|e| {
            crate::KmsError::Msg(format!("Failed to serialize final signed transaction: {e}"))
        })
    }

    /// Verifies an Ethereum EIP-155 signature for a given transaction hash and public key.
    ///
    /// Returns `Ok(true)` if the signature is valid, `Ok(false)` if invalid.
    /// Returns an error string for failures such as invalid formats.
    async fn verify(
        &mut self,
        transaction_hash_hex: &str,
        signature_hex: &str,
        key: &str,
    ) -> crate::Result<bool> {
        let key = self.resolve_key(key)?;
        let key_pair = {
            let keys = self.inner.keys.lock().await;
            keys.get(&key)
                .ok_or(crate::KmsError::PublicKeyNotFound)?
                .clone()
        };
        self.inner
            .verify_eip155(transaction_hash_hex, signature_hex, &key_pair.public_key)
            .map_err(|e| {
                let msg = format!("Signature verification failed: {e}");
                error!("{}", msg);
                crate::KmsError::Msg(msg)
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
    ) -> crate::Result<bool> {
        let key = self.resolve_key(key)?;
        let key_pair = {
            let keys = self.inner.keys.lock().await;
            keys.get(&key)
                .ok_or(crate::KmsError::PublicKeyNotFound)?
                .clone()
        };
        self.inner
            .verify_via_kms_eip155(transaction_hash_hex, signature_hex, &key_pair.public_key)
            .await
    }

    /// Deletes a public key from storage.
    ///
    /// Returns `Ok(true)` if the key was deleted, `Ok(false)` if the key was not found.
    /// Returns an error string if deletion fails.
    async fn delete_key(&mut self, key: &str) -> crate::Result<bool> {
        let key = self.resolve_key(key)?;
        Ok(self.inner.delete_key(&key).await)
    }

    /// Lists all stored keys along with their associated metadata.
    ///
    /// Returns a vector of `KeyEntry` on success.
    /// Returns an error string on failure.
    async fn list_keys(&mut self) -> crate::Result<Vec<KeyEntry>> {
        Ok(self.inner.list_keys().await)
    }
}

impl MockEthereumKeysService {
    /// Creates a new `MockEthereumKeysService` with an empty in-memory key store.
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
    pub fn new(config: Config, crypto_service: CryptoService) -> crate::Result<Self> {
        Ok(Self {
            inner: MockKeysService::new(config, crypto_service),
        })
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
            self.inner.crypto_service.address_eth(key).inspect_err(|e| {
                error!(error = %e, "Failed to convert public key to address");
            })
        } else {
            Ok(key.to_string())
        }
    }
}
