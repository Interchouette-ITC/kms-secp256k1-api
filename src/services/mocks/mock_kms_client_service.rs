#[cfg(test)]
use crate::constants::{
    CASPER_PUBLIC_KEY_PREFIXED, ETH_PUBLIC_KEY, ETH_SIGNATURE_BASE64, ETH_TRANSACTION_HASH,
    SIGNATURE_BASE64, TRANSACTION_HASH,
};
#[cfg(test)]
use crate::services::keys_service::KeyEntry;
#[cfg(test)]
use crate::services::kms_client_service::KmsClientService;
#[cfg(test)]
use base64::Engine;
#[cfg(test)]
use base64::engine::general_purpose::STANDARD;
#[cfg(test)]
use k256::{
    EncodedPoint, Secp256k1,
    ecdsa::SigningKey,
    elliptic_curve::{PublicKey, pkcs8::EncodePublicKey, rand_core::OsRng, sec1::FromEncodedPoint},
};
#[cfg(test)]
pub struct MockKmsClientService;
#[cfg(test)]
use crate::constants::CASPER_SECP_PREFIX;

#[cfg(test)]
#[async_trait::async_trait]
impl KmsClientService for MockKmsClientService {
    async fn create_key(&self) -> Result<(String, String), String> {
        let signing_key = SigningKey::random(&mut OsRng);
        let verifying_key = signing_key.verifying_key();
        let encoded_point: EncodedPoint = verifying_key.to_encoded_point(false);

        let public_key: PublicKey<Secp256k1> = PublicKey::from_encoded_point(&encoded_point)
            .into_option()
            .ok_or_else(|| "Failed to create PublicKey from encoded point".to_string())?;

        let der = public_key
            .to_public_key_der()
            .map_err(|e| format!("DER encode error: {e}"))?;

        let der_base64 = STANDARD.encode(der.as_bytes());

        Ok(("mock-key_id".to_string(), der_base64))
    }

    async fn create_alias(&self, _key_id: &str, _alias: &str) -> Result<(), String> {
        Ok(())
    }

    async fn sign(&self, _transaction_hash_hex: &str, public_key: &str) -> Result<String, String> {
        if public_key.contains(CASPER_PUBLIC_KEY_PREFIXED) {
            Ok(SIGNATURE_BASE64.to_string())
        } else if public_key.contains(ETH_PUBLIC_KEY) {
            Ok(ETH_SIGNATURE_BASE64.to_string())
        } else {
            unimplemented!("Mock KMS Client signature type unimplemented")
        }
    }

    async fn verify(
        &self,
        transaction_hash_hex: &str,
        signature_base64: &str,
        public_key: &str,
    ) -> Result<bool, String> {
        if (transaction_hash_hex.contains(TRANSACTION_HASH)
            && signature_base64.eq(SIGNATURE_BASE64)
            && public_key.contains(CASPER_PUBLIC_KEY_PREFIXED))
            || (transaction_hash_hex.contains(ETH_TRANSACTION_HASH)
                && signature_base64.eq(ETH_SIGNATURE_BASE64)
                && public_key.contains(ETH_PUBLIC_KEY))
        {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    async fn delete_key(&self, public_key: &str) -> Result<bool, String> {
        if public_key.contains("known_public_key") {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    async fn list_keys(&self) -> Result<Vec<KeyEntry>, String> {
        Ok(vec![
            KeyEntry {
                address: "address_1".to_string().into(),
                public_key_base64: STANDARD.encode("public_key_1_base64").into(),
                public_key: Some("public_key_1".to_string()).into(),
                key_id: "key_id_1".to_string().into(),
            },
            KeyEntry {
                address: "address_2".to_string().into(),
                public_key_base64: STANDARD.encode("public_key_2_base64").into(),
                public_key: Some("public_key_2".to_string()).into(),
                key_id: "key_id_2".to_string().into(),
            },
        ])
    }

    async fn get_public_key(&self, alias: &str) -> Result<String, String> {
        if alias.contains(&CASPER_PUBLIC_KEY_PREFIXED.replacen(CASPER_SECP_PREFIX, "", 1)) {
            Ok(CASPER_PUBLIC_KEY_PREFIXED.to_string())
        } else {
            Ok(ETH_PUBLIC_KEY.to_string())
        }
    }
}
