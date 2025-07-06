#[cfg(test)]
use crate::constants::{
    CASPER_PUBLIC_KEY_PREFIXED, ETH_PUBLIC_KEY, ETH_SIGNATURE_BASE64, ETH_SIGNATURE_BASE64_WITH_V,
    ETH_TRANSACTION_HASH, SIGNATURE_BASE64, TRANSACTION_HASH,
};
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

    async fn create_alias(&self, _key_id: &str, _public_key: &str) -> Result<(), String> {
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
        dbg!(signature_base64);
        if (transaction_hash_hex.contains(TRANSACTION_HASH)
            && signature_base64.eq(SIGNATURE_BASE64)
            && public_key.contains(CASPER_PUBLIC_KEY_PREFIXED))
            || (transaction_hash_hex.contains(ETH_TRANSACTION_HASH)
                && signature_base64.eq(ETH_SIGNATURE_BASE64_WITH_V)
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

    async fn list_keys(&self) -> Result<Vec<(String, String)>, String> {
        Ok(vec![
            ("key_id_1".to_string(), "public_key_1".to_string()),
            ("key_id_2".to_string(), "public_key_2".to_string()),
        ])
    }
}
