#[cfg(test)]
use crate::constants::{
    CASPER_PUBLIC_KEY_BASE64, CASPER_PUBLIC_KEY_PREFIXED, ETH_ADDRESS, ETH_PUBLIC_KEY,
    ETH_PUBLIC_KEY_BASE64, ETH_SIGNATURE_BASE64, ETH_TRANSACTION_HASH, SIGNATURE_BASE64,
    TRANSACTION_HASH,
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

    async fn sign(&self, _transaction_hash_hex: &str, address: &str) -> Result<String, String> {
        if address.eq(CASPER_PUBLIC_KEY_PREFIXED) {
            Ok(SIGNATURE_BASE64.to_string())
        } else if address.eq(ETH_ADDRESS) {
            Ok(ETH_SIGNATURE_BASE64.to_string())
        } else {
            unimplemented!("Mock KMS Client signature type unimplemented for sign")
        }
    }

    async fn verify(
        &self,
        transaction_hash_hex: &str,
        signature_base64: &str,
        key: &str,
    ) -> Result<bool, String> {
        if (transaction_hash_hex.contains(TRANSACTION_HASH)
            && signature_base64.eq(SIGNATURE_BASE64)
            && key.contains(CASPER_PUBLIC_KEY_PREFIXED))
            || (transaction_hash_hex.contains(ETH_TRANSACTION_HASH)
                && signature_base64.eq(ETH_SIGNATURE_BASE64)
                && key.eq(ETH_ADDRESS))
        {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    async fn delete_key(&self, key: &str) -> Result<bool, String> {
        if key.contains("known_public_key") {
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
        if CASPER_PUBLIC_KEY_PREFIXED.contains(alias) {
            Ok(CASPER_PUBLIC_KEY_BASE64.to_string())
        } else if alias.eq(ETH_ADDRESS) {
            Ok(ETH_PUBLIC_KEY_BASE64.to_string())
        } else if alias.eq("bad_key") {
            Ok(
                "MDYwEAYHKoZIzj0CAQYFK4EEAAoDIgAD97Il35cIXVY5dQimWRuWH9IYZ83coSENdDeaK3MjCIY="
                    .to_string(), // random key
            )
        } else {
            unimplemented!("Mock KMS Client signature type unimplemented for get_public_key")
        }
    }
}
