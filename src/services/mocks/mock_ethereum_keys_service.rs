use crate::{
    config::Config,
    services::{
        crypto_service::CryptoService,
        keys_service::KeysServiceTrait,
        mocks::mock_keys_service::{KeyPair, MockKeysService},
    },
};
use ethers::{
    signers::{LocalWallet, Signer},
    types::{H256, TransactionRequest, TxHash},
};
use k256::{
    Secp256k1,
    ecdsa::SigningKey,
    elliptic_curve::{PublicKey, rand_core::OsRng, sec1::FromEncodedPoint},
};
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
    async fn create_key(&mut self, _config: &Config) -> Result<String, String> {
        let signing_key = SigningKey::random(&mut OsRng);
        let verifying_key = signing_key.verifying_key();
        let encoded_point = verifying_key.to_encoded_point(true);

        let public_key = PublicKey::<Secp256k1>::from_encoded_point(&encoded_point)
            .into_option()
            .ok_or_else(|| "Failed to create PublicKey from encoded point".to_string())?;

        let public_key = hex::encode(public_key.to_sec1_bytes());

        let secret_key_bytes = signing_key.to_bytes();
        let secret_key_hex = hex::encode(secret_key_bytes);

        {
            let key_pair = KeyPair {
                public_key: public_key.clone(),
                secret_key: secret_key_hex,
            };
            let mut keys = self.inner.keys.lock().await;
            keys.entry(public_key.clone()).or_insert(key_pair);
        }
        Ok(public_key)
    }

    /// Signs a given transaction hash using the specified public key.
    ///
    /// Takes the transaction hash, public key, and configuration.
    /// Returns the signature as a hexadecimal string if successful.
    /// Returns an error string if signing fails.
    async fn sign_transaction_hash(
        &mut self,
        _config: &Config,
        transaction_hash: &str,
        public_key: &str,
    ) -> Result<String, String> {
        let key_pair = {
            let keys = self.inner.keys.lock().await;
            keys.get(public_key)
                .ok_or_else(|| "Public key not found".to_string())?
                .clone()
        };

        let tx_hash_bytes = hex::decode(transaction_hash)
            .map_err(|e| format!("Invalid transaction hash hex: {e}"))?;

        if tx_hash_bytes.len() != 32 {
            return Err("Transaction hash must be 32 bytes".into());
        }

        let tx_hash = H256::from_slice(&tx_hash_bytes);

        let wallet: LocalWallet = key_pair
            .secret_key
            .parse()
            .map_err(|e| format!("Failed to parse secret key into wallet: {e}"))?;

        let signature = wallet
            .sign_hash(tx_hash)
            .map_err(|e| format!("Failed to sign hash: {e}"))?;

        // let signature_test = self
        //     .inner
        //     .crypto_service
        //     .unconvert(&signature.to_string())
        //     .map_err(|e| {
        //         let msg = format!("Signature conversion failed: {e}");
        //         error!("{}", msg);
        //         msg
        //     })?;

        // info!(signature_test);

        // Verify signature
        let is_valid = self.verify(transaction_hash, &signature.to_string(), public_key)?;

        if !is_valid {
            return Err("Signature verification failed".to_string());
        }

        Ok(hex::encode(signature.to_vec()))
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
        let parsed: serde_json::Value = serde_json::from_str(transaction_str)
            .map_err(|e| format!("Failed to parse input JSON: {e}"))?;

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
            _ => return Err("Unsupported transaction format".to_string()),
        };

        // Deserialize transaction to struct for sighash
        let transaction: TransactionRequest = serde_json::from_value(transaction_value.clone())
            .map_err(|e| format!("Failed to parse transaction: {e}"))?;

        // Calculate sighash and validate
        let mut transaction_hash_str = format!("{:x}", transaction.sighash());
        transaction_hash_str = transaction_hash_str.trim_start_matches("0x").to_string();

        TxHash::from_str(&transaction_hash_str).map_err(|e| {
            error!("Invalid transaction hash: {:?}", e);
            "Invalid transaction hash".to_string()
        })?;

        // Get key pair and wallet
        let key_pair = {
            let keys = self.inner.keys.lock().await;
            keys.get(public_key)
                .ok_or_else(|| "Public key not found".to_string())?
                .clone()
        };

        let wallet: LocalWallet = key_pair
            .secret_key
            .parse()
            .map_err(|e| format!("Failed to parse secret key into wallet: {e}"))?;

        // Sign the transaction
        let signature = wallet
            .sign_transaction(&transaction.clone().into())
            .await
            .map_err(|e| format!("Failed to sign transaction: {e}"))?;

        let signature_hex = signature.to_string();

        // Verify signature
        let is_valid = self.verify(&transaction_hash_str, &signature_hex, public_key)?;
        if !is_valid {
            return Err("Generated signature failed verification".to_string());
        }

        // Append signature info
        signatures.push(json!({
            "signer": public_key,
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

        serde_json::to_string(&result)
            .map_err(|e| format!("Failed to serialize final signed transaction: {e}"))
    }

    /// Verifies an Ethereum EIP-155 signature for a given transaction hash and public key.
    ///
    /// Returns `Ok(true)` if the signature is valid, `Ok(false)` if invalid.
    /// Returns an error string for failures such as invalid formats.
    fn verify(
        &mut self,
        transaction_hash_hex: &str,
        signature_hex: &str,
        public_key: &str,
    ) -> Result<bool, String> {
        self.inner
            .verify_eip155(transaction_hash_hex, signature_hex, public_key)
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
        public_key: &str,
    ) -> Result<bool, String> {
        self.inner
            .verify_via_kms_eip155(transaction_hash_hex, signature_hex, public_key)
            .await
    }

    /// Deletes a public key from storage.
    ///
    /// Returns `Ok(true)` if the key was deleted, `Ok(false)` if the key was not found.
    /// Returns an error string if deletion fails.
    async fn delete_key(&mut self, public_key: &str) -> Result<bool, String> {
        Ok(self.inner.delete_key(public_key).await)
    }

    /// Lists all stored keys along with their associated metadata.
    ///
    /// Returns a vector of tuples `(public_key, metadata)` on success.
    /// Returns an error string on failure.
    async fn list_keys(&mut self) -> Result<Vec<(String, String)>, String> {
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
    pub async fn new(config: Config, crypto_service: CryptoService) -> Result<Self, String> {
        Ok(Self {
            inner: MockKeysService::new(config, crypto_service).await,
        })
    }
}
