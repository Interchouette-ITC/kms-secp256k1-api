use crate::{
    config::Config,
    constants::{CASPER_SECP_LEN, CASPER_SECP_PREFIX},
    services::{
        crypto_service::CryptoService,
        keys_service::{KeyEntry, KeysServiceTrait},
        mocks::mock_keys_service::{KeyPair, MockKeysService},
    },
};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use casper_rust_wasm_sdk::{
    SDK,
    helpers::{public_key_from_secret_key, secret_key_secp256k1_generate},
    types::{
        transaction::Transaction, transaction_params::transaction_str_params::TransactionStrParams,
    },
};
use regex::Regex;
use tracing::error;

pub struct MockCasperKeysService {
    inner: MockKeysService,
}

#[async_trait::async_trait]
impl KeysServiceTrait for MockCasperKeysService {
    async fn create_key(&mut self, _config: &Config) -> Result<KeyEntry, String> {
        let secret_key = secret_key_secp256k1_generate()
            .map_err(|e| format!("Failed to generate secret key: {e}"))?;
        let secret_key = secret_key.to_pem().map_err(|e| e.to_string())?;

        let address = public_key_from_secret_key(&secret_key)
            .map_err(|e| format!("Failed to get public key: {e}"))?; // generated key contains prefix

        let public_key = address.replacen(CASPER_SECP_PREFIX, "", 1).to_string(); // removes Casper prefix

        let key_id = public_key.clone();

        let public_key_base64 = STANDARD.encode(&public_key);

        // Store in map
        let key_pair = KeyPair {
            public_key: public_key.clone(),
            secret_key,
            address: address.clone(),
        };

        let mut keys = self.inner.keys.lock().await;
        keys.entry(address.clone()).or_insert(key_pair);

        Ok(KeyEntry {
            public_key: Some(public_key).into(),
            address: address.into(),
            public_key_base64: public_key_base64.into(),
            key_id: key_id.into(),
        })
    }

    async fn sign_transaction_hash(
        &mut self,
        config: &Config,
        transaction_hash: &str,
        key: &str,
    ) -> Result<String, String> {
        let key = self.resolve_key(key)?;
        let key_pair = {
            let keys = self.inner.keys.lock().await;
            keys.get(&key)
                .ok_or_else(|| "Public key not found".to_string())?
                .clone()
        };

        let public_key = &key_pair.address;

        let transaction_params = TransactionStrParams::default();
        transaction_params.set_chain_name("casper-net-1");
        transaction_params.set_initiator_addr(public_key);
        transaction_params.set_payment_amount("100000000");
        let sdk = SDK::new(None, None, None);
        let make_transfer = sdk
            .make_transfer_transaction(None, public_key, "2500000000", transaction_params, None)
            .map_err(|e| format!("Failed to create transfer transaction: {e}"))?;

        let mut transaction_str = make_transfer.to_json_string().unwrap_or_default();

        let re = Regex::new(r#""hash"\s*:\s*"[^"]+""#).unwrap();
        transaction_str = re
            .replace(&transaction_str, format!(r#""hash": "{transaction_hash}""#))
            .to_string();

        let signed_tx = self
            .sign_transaction(config, &transaction_str, public_key)
            .await
            .unwrap_or_default();

        let transaction: Transaction = Transaction::from_json_string(&signed_tx)
            .map_err(|e| format!("Failed to parse transaction: {e}"))?;

        Ok(transaction
            .approvals()
            .first()
            .unwrap()
            .signature()
            .to_hex_string())
    }

    async fn sign_transaction(
        &mut self,
        _config: &Config,
        transaction_str: &str,
        key: &str,
    ) -> Result<String, String> {
        let mut transaction: Transaction = Transaction::from_json_string(transaction_str)
            .map_err(|e| format!("Failed to parse transaction: {e}"))?;

        let key = self.resolve_key(key)?;
        let key_pair = {
            let keys = self.inner.keys.lock().await;
            keys.get(&key)
                .ok_or_else(|| "Public key not found".to_string())?
                .clone()
        };

        let signed_tx = transaction.sign(&key_pair.secret_key);

        Ok(signed_tx.to_json_string().unwrap_or_default())
    }

    async fn verify(
        &mut self,
        transaction_hash_hex: &str,
        signature_hex: &str,
        key: &str,
    ) -> Result<bool, String> {
        let key = self.resolve_key(key)?;
        self.inner
            .verify(transaction_hash_hex, signature_hex, &key)
            .map_err(|e| {
                let msg = format!("Signature verification failed: {e}");
                error!("{}", msg);
                msg
            })
    }

    async fn verify_via_kms(
        &mut self,
        transaction_hash_hex: &str,
        signature_hex: &str,
        key: &str,
    ) -> Result<bool, String> {
        let key = self.resolve_key(key)?;
        self.inner
            .verify_via_kms(transaction_hash_hex, signature_hex, &key)
            .await
    }

    async fn delete_key(&mut self, key: &str) -> Result<bool, String> {
        let key = self.resolve_key(key)?;
        Ok(self.inner.delete_key(&key).await)
    }

    async fn list_keys(&mut self) -> Result<Vec<KeyEntry>, String> {
        Ok(self.inner.list_keys().await)
    }
}

impl MockCasperKeysService {
    /// Creates a new `MockCasperKeysService` with an empty in-memory key store.
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

    /// Resolves a given key string to a Casper address format.
    ///
    /// This function checks whether the input `key` is a full Casper-compatible public key (by
    /// comparing its length to the expected `CASPER_SECP_LEN`). If so, it returns the key as-is.
    /// Otherwise, it assumes the input is a truncated or raw key, and prefixes it with
    /// `CASPER_SECP_PREFIX` to form a valid Casper address format.
    ///
    /// # Parameters
    /// - `key`: A string that is either a full-length Casper-compatible key or a truncated key.
    ///
    /// # Returns
    /// - `Ok(String)`: The resolved Casper address string.
    /// - `Err(String)`: This implementation does not return errors, but the signature allows for future error handling.
    ///
    /// # Behavior
    /// - If `key.len() == CASPER_SECP_LEN`, the input is returned directly.
    /// - Otherwise, the key is prefixed with `CASPER_SECP_PREFIX` and returned.
    fn resolve_key(&mut self, key: &str) -> Result<String, String> {
        if key.len() == CASPER_SECP_LEN {
            Ok(key.to_string())
        } else {
            Ok(format!("{CASPER_SECP_PREFIX}{key}"))
        }
    }
}
