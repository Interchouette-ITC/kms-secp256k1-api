use crate::{
    config::Config,
    constants::{COSMOS_SECP_LEN, DEFAULT_COSMOS_HRP},
    services::{
        cosmos_keys_service::{
            BodyHelper, append_signature_to_transaction, build_auth_info, fee_amount_json,
            fetch_account_info, signature_to_json,
        },
        crypto_service::CryptoService,
        keys_service::{KeyEntry, KeysServiceTrait, SigEntry},
        mocks::mock_keys_service::{KeyPair, MockKeysService},
    },
};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use cosmrs::{
    bip32::{PrivateKey, PublicKey},
    proto::cosmos::tx::v1beta1::TxRaw,
    tendermint::chain::Id,
    tx::{Fee, MessageExt, SignDoc},
};
use k256::{
    ecdsa::{Signature, SigningKey, signature::Signer},
    elliptic_curve::rand_core::OsRng,
    sha2::{Digest, Sha256},
};
use serde_json::json;
use std::str::FromStr;
use tracing::error;

pub struct MockCosmosKeysService {
    inner: MockKeysService,
    hrp: String,
}

#[async_trait::async_trait]
impl KeysServiceTrait for MockCosmosKeysService {
    async fn create_key(&mut self, _config: &Config) -> Result<KeyEntry, String> {
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
    ) -> Result<SigEntry, String> {
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

        let private_key_bytes = hex::decode(&key_pair.private_key)
            .map_err(|e| format!("Failed to decode secret key: {e}"))?;

        let signing_key = SigningKey::from_slice(&private_key_bytes)
            .map_err(|e| format!("Failed to create signing key: {e}"))?;

        let signature: Signature = signing_key.sign(&hash_bytes);
        let der = signature.to_der();
        let der_bytes = der.as_bytes();
        let base64_signature = STANDARD.encode(der_bytes);
        // let sig_hex_long = hex::encode(der_bytes);

        let signature = self
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
            .verify(transaction_hash, &signature, &key_pair.public_key)
            .await?;

        if !is_valid {
            return Err("Signature verification failed".to_string());
        }

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
    #[allow(clippy::too_many_lines)]
    async fn sign_transaction(
        &mut self,
        config: &Config,
        transaction_str: &str,
        key: &str,
    ) -> Result<String, String> {
        let tx_json: serde_json::Value = serde_json::from_str(transaction_str)
            .map_err(|e| format!("Failed to parse JSON: {e}"))?;

        // Extract chain_id and address
        let chain_id = Id::from_str(&config.get_cosmos_chain_id())
            .map_err(|e| format!("Failed to fetch chain_id: {e}"))?;

        // Get key pair and wallet
        let key = self.resolve_key(key)?;
        let key_pair = {
            let keys = self.inner.keys.lock().await;
            keys.get(&key)
                .ok_or_else(|| "Public key not found".to_string())?
                .clone()
        };
        let key = key_pair.address;

        // Deserialize TxBody
        let helper: BodyHelper = serde_json::from_value(tx_json["body"].clone())
            .map_err(|e| format!("Invalid TxBody: {e}"))?;

        let tx_body = helper.into_body()?;

        // Deserialize Fee
        let fee: Fee = serde_json::from_value(tx_json["auth_info"]["fee"].clone())
            .map_err(|e| format!("Invalid Fee: {e}"))?;

        let private_key_bytes = hex::decode(&key_pair.private_key)
            .map_err(|e| format!("Failed to decode secret key: {e}"))?;

        let signing_key = SigningKey::from_slice(&private_key_bytes)
            .map_err(|e| format!("Failed to create signing key: {e}"))?;

        let public_key = signing_key.public_key();

        let public_key_bytes = public_key.to_bytes();
        let pubkey_base64 = STANDARD.encode(public_key_bytes);

        // Fetch account_number and sequence
        let mut account = fetch_account_info(&key, &pubkey_base64, config)
            .await
            .map_err(|e| format!("Failed to fetch account info: {e}"))?;

        let Some(fetched_pub_key) = account.pub_key.take() else {
            return Err("Account info is missing pub_key".to_string());
        };

        if fetched_pub_key.key != pubkey_base64 {
            return Err(format!(
                "Invalid fetched public key: got {}",
                fetched_pub_key.key
            ));
        }
        let auth_info = build_auth_info(&public_key_bytes, account.sequence, &fee)?;

        // Build SignDoc
        let sign_doc = SignDoc::new(&tx_body, &auth_info, &chain_id, account.sequence)
            .map_err(|e| format!("SignDoc error: {e}"))?;

        let sign_doc_bytes = sign_doc
            .into_bytes()
            .map_err(|e| format!("SignDoc encode error: {e}"))?;

        // Hash and sign
        let transaction_hash = Sha256::digest(&sign_doc_bytes);
        let signature: Signature = signing_key.sign(&transaction_hash);
        let transaction_hash = hex::encode(transaction_hash);

        // Verify signature
        let signature_hex = signature.to_string();
        let public_key = hex::encode(public_key_bytes);

        let is_valid = self
            .verify(&transaction_hash, &signature_hex, &public_key)
            .await?;
        if !is_valid {
            return Err("Generated signature failed verification".to_string());
        }

        let body_bytes = tx_body
            .into_bytes()
            .map_err(|e| format!("Failed to encode body: {e}"))?;
        let auth_info_bytes = auth_info
            .into_bytes()
            .map_err(|e| format!("Failed to encode auth_info: {e}"))?;

        let tx_raw = TxRaw {
            body_bytes,
            auth_info_bytes,
            signatures: vec![signature.to_bytes().to_vec()],
        };

        let tx_raw_bytes = tx_raw
            .to_bytes()
            .map_err(|e| format!("Failed to encode TxRaw: {e}"))?;

        let base64_tx = STANDARD.encode(tx_raw_bytes);
        let broadcast_request = json!({
            "tx_bytes": base64_tx,
            "mode": "BROADCAST_MODE_SYNC"  // or "BLOCK" or "ASYNC"
        });
        let fee_amount_json = fee_amount_json(&fee);

        // Existing signatures from the original transaction (if any)
        let new_signature = signature_to_json(&key, &public_key, &signature, &transaction_hash);
        let signatures_array = append_signature_to_transaction(&tx_json, new_signature);

        // Prepare final JSON response
        let result = json!({
            "chain_id": chain_id,
            "body": tx_json["body"],
            "auth_info": {
                "signer_infos": [
                    {
                        "public_key": {
                            "@type": fetched_pub_key.key_type,
                            "key": fetched_pub_key.key,
                        },
                        "mode_info": {
                            "single": { "mode": "SIGN_MODE_DIRECT" }
                        },
                        "sequence": account.sequence,
                        "account_number": account.account_number
                    }
                ],
                "fee": {
                    "amount": fee_amount_json,
                    "gas_limit": fee.gas_limit
                }
            },
            "broadcast_request": broadcast_request,
            "signatures": signatures_array
        });

        // Return the wrapped transaction + signatures JSON
        serde_json::to_string(&result)
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
    /// Returns a vector of `KeyEntry` on success.
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
    pub fn new(config: Config, crypto_service: CryptoService) -> Result<Self, String> {
        let cosmos_hrp = config.get_cosmos_hrp();
        let hrp = match cosmos_hrp.as_str() {
            "" => DEFAULT_COSMOS_HRP,
            hrp => hrp,
        }
        .to_string();

        Ok(Self {
            inner: MockKeysService::new(config, crypto_service),
            hrp,
        })
    }

    /// Resolves a given key string to a Cosmos Bech32 address.
    ///
    /// This function checks whether the input `key` is a compressed secp256k1 public key
    /// (by comparing its length to the expected `COSMOS_SECP_LEN`). If so, it attempts to convert
    /// the public key to its corresponding Cosmos address using the underlying crypto service and the configured `hrp`.
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
    /// - The Bech32 address is generated using the provided `hrp` as prefix.
    /// - This function logs an error internally if conversion fails.
    fn resolve_key(&mut self, key: &str) -> Result<String, String> {
        if key.len() == COSMOS_SECP_LEN {
            self.inner
                .crypto_service
                .address_cosmos(key, &self.hrp)
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
