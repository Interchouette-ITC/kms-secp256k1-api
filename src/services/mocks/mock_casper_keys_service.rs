use crate::{
    config::Config,
    services::{
        crypto_service::CryptoService,
        keys_service::KeysServiceTrait,
        mocks::mock_keys_service::{KeyPair, MockKeysService},
    },
};
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
    async fn create_key(&mut self, _config: &Config) -> Result<String, String> {
        let secret_key = secret_key_secp256k1_generate()
            .map_err(|e| format!("Failed to generate secret key: {e}"))?;
        let secret_key = secret_key.to_pem().map_err(|e| e.to_string())?;
        let public_key = public_key_from_secret_key(&secret_key)
            .map_err(|e| format!("Failed to get public key: {e}"))?;

        {
            let key_pair = KeyPair {
                public_key: public_key.clone(),
                secret_key,
            };
            let mut keys = self.inner.keys.lock().await;
            keys.entry(public_key.clone()).or_insert(key_pair);
        }

        Ok(public_key)
    }

    async fn sign_transaction_hash(
        &mut self,
        config: &Config,
        transaction_hash: &str,
        public_key: &str,
    ) -> Result<String, String> {
        let transaction_params = TransactionStrParams::default();
        transaction_params.set_chain_name("mock");
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
        public_key: &str,
    ) -> Result<String, String> {
        let mut transaction: Transaction = Transaction::from_json_string(transaction_str)
            .map_err(|e| format!("Failed to parse transaction: {e}"))?;

        let key_pair = {
            let keys = self.inner.keys.lock().await;
            keys.get(public_key)
                .ok_or_else(|| "Public key not found".to_string())?
                .clone()
        };

        let signed_tx = transaction.sign(&key_pair.secret_key);

        Ok(signed_tx.to_json_string().unwrap_or_default())
    }

    fn verify(
        &mut self,
        transaction_hash_hex: &str,
        signature_hex: &str,
        public_key: &str,
    ) -> Result<bool, String> {
        self.inner
            .verify(transaction_hash_hex, signature_hex, public_key)
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
        public_key: &str,
    ) -> Result<bool, String> {
        self.inner
            .verify_via_kms(transaction_hash_hex, signature_hex, public_key)
            .await
    }

    async fn delete_key(&mut self, public_key: &str) -> Result<bool, String> {
        Ok(self.inner.delete_key(public_key).await)
    }

    async fn list_keys(&mut self) -> Result<Vec<(String, String)>, String> {
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
}
