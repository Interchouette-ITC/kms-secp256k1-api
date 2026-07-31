#[cfg(test)]
use crate::constants::{
    CASPER_PUBLIC_KEY_BASE64, CASPER_PUBLIC_KEY_PREFIXED, COSMOS_ADDRESS, COSMOS_PUBLIC_KEY_BASE64,
    COSMOS_SIGNATURE_BASE64, COSMOS_TRANSACTION_HASH, ETH_ADDRESS, ETH_PUBLIC_KEY_BASE64,
    ETH_SIGNATURE_BASE64, ETH_TRANSACTION_HASH, RANDOM_PUBLIC_KEY_BASE64, SIGNATURE_BASE64,
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
    async fn create_key(&self) -> crate::Result<(String, String)> {
        let signing_key = SigningKey::random(&mut OsRng);
        let verifying_key = signing_key.verifying_key();
        let encoded_point: EncodedPoint = verifying_key.to_encoded_point(false);

        let public_key: PublicKey<Secp256k1> = PublicKey::from_encoded_point(&encoded_point)
            .into_option()
            .ok_or_else(|| {
                crate::KmsError::Msg("Failed to create PublicKey from encoded point".into())
            })?;

        let der = public_key
            .to_public_key_der()
            .map_err(|e| crate::KmsError::Msg(format!("DER encode error: {e}")))?;

        let der_base64 = STANDARD.encode(der.as_bytes());

        Ok(("mock-key_id".to_string(), der_base64))
    }

    async fn create_alias(&self, _key_id: &str, _alias: &str) -> crate::Result<()> {
        Ok(())
    }

    async fn sign(&self, _transaction_hash_hex: &str, address: &str) -> crate::Result<String> {
        if address.eq(CASPER_PUBLIC_KEY_PREFIXED) {
            Ok(SIGNATURE_BASE64.to_string())
        } else if address.eq(ETH_ADDRESS) {
            Ok(ETH_SIGNATURE_BASE64.to_string())
        } else if address.eq(COSMOS_ADDRESS) {
            Ok(COSMOS_SIGNATURE_BASE64.to_string())
        } else {
            unimplemented!("Mock KMS Client signature type unimplemented for sign")
        }
    }

    async fn verify(
        &self,
        transaction_hash_hex: &str,
        signature_base64: &str,
        key: &str,
    ) -> crate::Result<bool> {
        let expected_cases = [
            (
                TRANSACTION_HASH,
                SIGNATURE_BASE64,
                CASPER_PUBLIC_KEY_PREFIXED,
            ),
            (ETH_TRANSACTION_HASH, ETH_SIGNATURE_BASE64, ETH_ADDRESS),
            (
                COSMOS_TRANSACTION_HASH,
                COSMOS_SIGNATURE_BASE64,
                COSMOS_ADDRESS,
            ),
        ];

        let matches = expected_cases
            .iter()
            .any(|(tx_hash, signature, match_key)| {
                (transaction_hash_hex.contains(tx_hash)
                    && signature_base64 == *signature
                    && key.contains(match_key))
                    || key == *match_key
            });

        Ok(matches)
    }

    async fn delete_key(&self, key: &str) -> crate::Result<bool> {
        if key.contains("known_public_key") {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    async fn list_keys(&self) -> crate::Result<Vec<KeyEntry>> {
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

    async fn get_public_key(&self, alias: &str) -> crate::Result<String> {
        if CASPER_PUBLIC_KEY_PREFIXED.contains(alias) {
            Ok(CASPER_PUBLIC_KEY_BASE64.to_string())
        } else if alias.eq(ETH_ADDRESS) {
            Ok(ETH_PUBLIC_KEY_BASE64.to_string())
        } else if alias.eq(COSMOS_ADDRESS) {
            Ok(COSMOS_PUBLIC_KEY_BASE64.to_string())
        } else if alias.eq("bad_key") {
            Ok(
                RANDOM_PUBLIC_KEY_BASE64.to_string(), // random key
            )
        } else {
            unimplemented!("Mock KMS Client signature type unimplemented for get_public_key")
        }
    }
}
