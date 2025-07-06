use crate::{
    config::{AwsConfig, HashType},
    services::kms_client_service::KmsClientService,
};
use aws_config::SdkConfig;
use aws_credential_types::Credentials;
use aws_sdk_kms::{
    Client as KmsClient,
    primitives::Blob,
    types::{KeySpec, KeyUsageType, MessageType, OriginType, SigningAlgorithmSpec},
};
use aws_types::region::Region;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use k256::sha2::Sha256;
use sha3::{Digest as Sha3Digest, Sha3_256};
use std::vec;
use tracing::{error, info};

pub struct AWSKmsClientService {
    create: KmsClient,
    sign: KmsClient,
    delete: Option<KmsClient>,
    list: Option<KmsClient>,
    pub hash_type: HashType,
}

impl AWSKmsClientService {
    /// Creates a new `AWSKmsClientService` with the provided AWS credentials.
    ///
    /// # Errors
    ///
    /// Returns an error string if the SDK configuration fails.
    pub async fn new(aws_config: AwsConfig) -> Result<Self, String> {
        let region = Region::new(aws_config.region.clone());

        let create_creds = Credentials::new(
            aws_config.create.access_key_id,
            aws_config.create.secret_access_key,
            None,
            None,
            "create_kms_credentials",
        );

        let sign_creds = Credentials::new(
            aws_config.sign.access_key_id,
            aws_config.sign.secret_access_key,
            None,
            None,
            "sign_kms_credentials",
        );

        let create_sdk_config = aws_config_to_sdk_config(region.clone(), create_creds).await;
        let sign_sdk_config = aws_config_to_sdk_config(region.clone(), sign_creds).await;

        let create = KmsClient::new(&create_sdk_config);
        let sign = KmsClient::new(&sign_sdk_config);

        // Conditionally build delete if credentials exist
        let delete = if let Some(delete_creds) = &aws_config.delete {
            let delete_creds = Credentials::new(
                delete_creds.access_key_id.clone(),
                delete_creds.secret_access_key.clone(),
                None,
                None,
                "delete_kms_credentials",
            );
            let delete_sdk_config = aws_config_to_sdk_config(region.clone(), delete_creds).await;
            Some(KmsClient::new(&delete_sdk_config))
        } else {
            None
        };

        let list = if let Some(list_creds) = &aws_config.list {
            let delete_creds = Credentials::new(
                list_creds.access_key_id.clone(),
                list_creds.secret_access_key.clone(),
                None,
                None,
                "list_kms_credentials",
            );
            let delete_sdk_config = aws_config_to_sdk_config(region.clone(), delete_creds).await;
            Some(KmsClient::new(&delete_sdk_config))
        } else {
            None
        };

        Ok(Self {
            create,
            sign,
            delete,
            list,
            hash_type: aws_config.hash_type,
        })
    }

    // Casper	SHA256 (x2)	RAW	Transaction hash, AWS re-hashes with SHA256
    // Ethereum (EIP-155) Keccak256	DIGEST, AWS does not re-hashes
    // Other	SHA3-256 DIGEST, AWS does not re-hashes
    fn hash(&self, data: &[u8]) -> Vec<u8> {
        match self.hash_type {
            HashType::Sha256 => {
                let mut hasher = Sha256::new();
                hasher.update(data);
                hasher.finalize().to_vec() // Casper Sha256 of Sha256 transaction_hash
            }
            HashType::Keccak256 => data.to_vec(), // EIP-155 transaction hash is Keccak256 hash already
            HashType::Sha3_256 => {
                let mut hasher = Sha3_256::new();
                hasher.update(data);
                hasher.finalize().to_vec()
            }
        }
    }
}

#[async_trait::async_trait]
impl KmsClientService for AWSKmsClientService {
    async fn create_key(&self) -> Result<(String, String), String> {
        let kms_client = &self.create;

        let create_key_output = kms_client
            .create_key()
            .description("SECP256K1 Key")
            .key_usage(KeyUsageType::SignVerify)
            .origin(OriginType::AwsKms)
            .key_spec(KeySpec::EccSecgP256K1)
            .send()
            .await
            .map_err(|e| {
                let msg = format!("Error creating key: {e:?}");
                error!("{}", &msg);
                msg
            })?;

        let key_id = create_key_output
            .key_metadata
            .map(|meta| meta.key_id)
            .ok_or_else(|| {
                let msg = "No KeyId found".to_string();
                error!("{}", &msg);
                msg
            })?;

        info!("Key created: {}", key_id);

        // Fetch the public key
        let get_public_key_resp = kms_client
            .get_public_key()
            .key_id(&key_id)
            .send()
            .await
            .map_err(|e| {
                let msg = format!("Error fetching public key: {e:?}");
                error!("{}", &msg);
                msg
            })?;

        let public_key = get_public_key_resp
            .public_key
            .as_ref()
            .ok_or_else(|| {
                let msg = "No public key returned by AWS KMS".to_string();
                error!("{}", &msg);
                msg
            })?
            .as_ref();

        let public_key = STANDARD.encode(public_key);
        // info!("Public key (base64): {}", public_key);
        Ok((key_id, public_key))
    }

    async fn create_alias(&self, key_id: &str, public_key: &str) -> Result<(), String> {
        let alias_name = format!("alias/{public_key}");
        let kms_client = &self.create;

        kms_client
            .create_alias()
            .alias_name(&alias_name)
            .target_key_id(key_id)
            .send()
            .await
            .map_err(|e| {
                let msg = format!("Error creating alias: {e:?}");
                error!("{}", &msg);
                msg
            })?;
        //   info!("Alias key: {}", alias_name);
        Ok(())
    }

    async fn sign(&self, transaction_hash_hex: &str, public_key: &str) -> Result<String, String> {
        let kms_client = &self.sign;
        let alias_name = format!("alias/{public_key}");

        let describe_key_output = kms_client
            .describe_key()
            .key_id(&alias_name)
            .send()
            .await
            .map_err(|e| {
                let msg = format!("Failed to describe key for alias {alias_name}: {e:?}");
                error!("{}", msg);
                msg
            })?;

        let key_metadata = describe_key_output.key_metadata.ok_or_else(|| {
            let msg = format!("KeyMetadata not found for alias {alias_name}");
            error!("{}", msg);
            msg
        })?;

        let key_id = key_metadata.key_id;

        let data = hex::decode(transaction_hash_hex).map_err(|e| {
            let msg = format!("Failed to decode transaction hash hex: {e}");
            error!("{}", msg);
            msg
        })?;

        let transaction_hash = STANDARD.encode(data.clone());
        info!("transaction_hash: {}", transaction_hash);

        let input_data = self.hash(&data);

        // Sign the message
        let sign_output = kms_client
            .sign()
            .key_id(&key_id)
            .message(Blob::new(input_data))
            .message_type(MessageType::Digest)
            .signing_algorithm(SigningAlgorithmSpec::EcdsaSha256)
            .send()
            .await
            .map_err(|e| {
                let msg = format!("Error calling sign on KMS: {e:?}");
                error!("{}", msg);
                msg
            })?;

        let signature = sign_output.signature.ok_or_else(|| {
            let msg = "No signature returned from AWS KMS".to_string();
            error!("{}", msg);
            msg
        })?;

        let signature = signature.as_ref();

        // Log signature
        let signature_hex = hex::encode(signature);
        let signature = STANDARD.encode(signature);
        info!("signature: {}", signature);
        info!("signature: {}", signature_hex);

        Ok(signature)
    }

    async fn verify(
        &self,
        transaction_hash_hex: &str,
        signature_asn1_base64: &str,
        public_key: &str,
    ) -> Result<bool, String> {
        let kms_client = &self.sign;
        let alias_name = format!("alias/{public_key}");

        // Resolve alias to key ID
        let describe_key_output = kms_client
            .describe_key()
            .key_id(&alias_name)
            .send()
            .await
            .map_err(|e| {
                let msg = format!("Failed to describe key for alias {alias_name}: {e:?}");
                error!("{}", msg);
                msg
            })?;

        let key_metadata = describe_key_output.key_metadata.ok_or_else(|| {
            let msg = format!("KeyMetadata not found for alias {alias_name}");
            error!("{}", msg);
            msg
        })?;

        let key_id = key_metadata.key_id;

        // Decode the digest hex to bytes
        let digest_bytes = hex::decode(transaction_hash_hex).map_err(|e| {
            let msg = format!("Failed to decode transaction hash hex: {e}");
            error!("{}", msg);
            msg
        })?;

        // Decode the signature base64 to bytes (ASN.1 DER format expected)
        let signature_bytes = STANDARD.decode(signature_asn1_base64).map_err(|e| {
            let msg = format!("Failed to decode signature base64: {e}");
            error!("{}", msg);
            msg
        })?;

        let input_data = self.hash(&digest_bytes);

        // Call AWS KMS verify
        let verify_output = kms_client
            .verify()
            .key_id(&key_id)
            .message(Blob::new(input_data))
            .message_type(MessageType::Digest)
            .signature(Blob::new(signature_bytes))
            .signing_algorithm(SigningAlgorithmSpec::EcdsaSha256)
            .send()
            .await
            .map_err(|e| {
                let msg = format!("Error calling verify on KMS: {e:?}");
                error!("{}", msg);
                msg
            })?;

        // The 'signature_valid' field indicates validity
        Ok(verify_output.signature_valid)
    }

    async fn delete_key(&self, public_key: &str) -> Result<bool, String> {
        let Some(kms_client) = &self.delete else {
            tracing::warn!(
                "Attempted to delete key, but delete_kms client is not configured/enabled"
            );
            return Ok(false);
        };

        let alias_name = format!("alias/{public_key}");

        let describe_output = kms_client
            .describe_key()
            .key_id(&alias_name)
            .send()
            .await
            .map_err(|e| {
                let msg = format!("Failed to describe key for alias {alias_name}: {e:?}");
                error!("{}", msg);
                msg
            })?;

        let key_metadata = describe_output.key_metadata.ok_or_else(|| {
            let msg = format!("No key metadata found for alias {alias_name}");
            error!("{}", msg);
            msg
        })?;

        let key_id = key_metadata.key_id.clone();

        kms_client
            .delete_alias()
            .alias_name(&alias_name)
            .send()
            .await
            .map_err(|e| {
                let msg = format!("Failed to delete alias {alias_name}: {e:?}");
                error!("{}", msg);
                msg
            })?;

        kms_client
            .schedule_key_deletion()
            .key_id(&key_id)
            .pending_window_in_days(7)
            .send()
            .await
            .map_err(|e| {
                let msg = format!("Failed to schedule deletion for key {key_id}: {e:?}");
                error!("{}", msg);
                msg
            })?;

        info!(
            "Key {} (alias {}) scheduled for deletion",
            key_id, alias_name
        );
        Ok(true)
    }

    async fn list_keys(&self) -> Result<Vec<(String, String)>, String> {
        let Some(kms_client) = &self.list else {
            tracing::warn!("Attempted to list keys, but list_kms client is not configured/enabled");
            return Ok(vec![]);
        };

        let mut paginator = kms_client.list_aliases().into_paginator().send();
        let mut results = Vec::new();

        while let Some(page) = paginator.next().await {
            let page = page.map_err(|e| {
                let msg = format!("Failed to list aliases: {e:?}");
                error!("{}", msg);
                msg
            })?;

            if let Some(aliases) = page.aliases {
                for alias in aliases {
                    if let (Some(alias_name), Some(target_key_id)) =
                        (alias.alias_name, alias.target_key_id)
                        && !alias_name.starts_with("alias/aws/")
                    {
                        let cleaned_alias = alias_name.trim_start_matches("alias/").to_string();
                        results.push((cleaned_alias, target_key_id));
                    }
                }
            }
        }

        //  info!("Found {} client key aliases", results.len());
        Ok(results)
    }
}

async fn aws_config_to_sdk_config(region: Region, creds: Credentials) -> SdkConfig {
    let config_loader = aws_config::defaults(aws_config::BehaviorVersion::latest());
    config_loader
        .region(region)
        .credentials_provider(creds)
        .load()
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AwsConfig, AwsCreds};

    #[tokio::test]
    async fn test_aws_kms_client_service_new_success() {
        let dummy_creds = AwsCreds {
            access_key_id: "dummy_access_key".into(),
            secret_access_key: "dummy_secret_key".into(),
        };

        let aws_config = AwsConfig {
            region: "us-east-1".into(),
            create: dummy_creds.clone(),
            sign: dummy_creds.clone(),
            delete: None,
            list: None,
            ..Default::default()
        };

        let result = AWSKmsClientService::new(aws_config).await;

        assert!(
            result.is_ok(),
            "Service construction failed: {:?}",
            result.err()
        );

        let service = result.unwrap();

        assert!(service.delete.is_none());
        assert!(service.list.is_none());
        assert_eq!(service.hash_type, HashType::Sha256);
    }

    #[tokio::test]
    async fn test_aws_kms_client_service_new_success_list_delete() {
        let dummy_creds = AwsCreds {
            access_key_id: "dummy_access_key".into(),
            secret_access_key: "dummy_secret_key".into(),
        };

        let aws_config = AwsConfig {
            region: "us-east-1".into(),
            create: dummy_creds.clone(),
            sign: dummy_creds.clone(),
            delete: Some(dummy_creds.clone()),
            list: Some(dummy_creds.clone()),
            hash_type: HashType::Keccak256,
        };

        let result = AWSKmsClientService::new(aws_config).await;

        assert!(
            result.is_ok(),
            "Service construction failed: {:?}",
            result.err()
        );

        let service = result.unwrap();

        assert!(service.delete.is_some());
        assert!(service.list.is_some());
        assert_eq!(service.hash_type, HashType::Keccak256);
    }
}
