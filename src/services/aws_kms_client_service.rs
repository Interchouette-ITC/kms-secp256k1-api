use crate::{
    config::{AwsConfig, HashType},
    services::{keys_service::KeyEntry, kms_client_service::KmsClientService},
};
use aws_config::SdkConfig;
use aws_credential_types::Credentials;
use aws_sdk_kms::{
    Client as KmsClient,
    primitives::Blob,
    types::{KeyMetadata, KeySpec, KeyUsageType, MessageType, OriginType, SigningAlgorithmSpec},
};
use aws_types::region::Region;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use k256::sha2::{Digest, Sha256};
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
    pub async fn new(aws_config: AwsConfig) -> crate::Result<Self> {
        let region = Region::new(aws_config.region.clone());
        let endpoint = aws_config.endpoint.clone();

        // Log AWS configuration for debugging
        info!(
            "Initializing AWS KMS Client - Region: {}, Endpoint: {}",
            aws_config.region, endpoint
        );

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

        let create_sdk_config =
            aws_config_to_sdk_config(region.clone(), endpoint.clone(), create_creds).await;
        let sign_sdk_config =
            aws_config_to_sdk_config(region.clone(), endpoint.clone(), sign_creds).await;

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
            let delete_sdk_config =
                aws_config_to_sdk_config(region.clone(), endpoint.clone(), delete_creds).await;
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
            let delete_sdk_config =
                aws_config_to_sdk_config(region.clone(), endpoint, delete_creds).await;
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
        }
    }
}

#[async_trait::async_trait]
impl KmsClientService for AWSKmsClientService {
    async fn create_key(&self) -> crate::Result<(String, String)> {
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
                crate::KmsError::Msg(msg)
            })?;

        let key_id = create_key_output
            .key_metadata
            .map(|meta| meta.key_id)
            .ok_or_else(|| {
                error!("No KeyId found");
                crate::KmsError::MissingKeyId
            })?;

        // Fetch the public key
        let public_key_base64 = self.get_public_key_base64(kms_client, &key_id).await?;

        Ok((key_id, public_key_base64))
    }

    async fn create_alias(&self, key_id: &str, alias: &str) -> crate::Result<()> {
        let alias_name = Self::format_alias(alias);
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
                crate::KmsError::Msg(msg)
            })?;
        Ok(())
    }

    async fn sign(&self, transaction_hash_hex: &str, key: &str) -> crate::Result<String> {
        let kms_client = &self.sign;

        // Resolve key to key ID
        let key_metadata = self.describe_key_metadata(kms_client, key).await?;
        let key_id = key_metadata.key_id;

        let data = hex::decode(transaction_hash_hex).map_err(|e| {
            let msg = format!("Failed to decode transaction hash hex: {e}");
            error!("{}", msg);
            crate::KmsError::Msg(msg)
        })?;

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
                crate::KmsError::Msg(msg)
            })?;

        let signature = sign_output.signature.ok_or_else(|| {
            let msg = "No signature returned from AWS KMS".to_string();
            error!("{}", msg);
            crate::KmsError::Msg(msg)
        })?;

        let signature = signature.as_ref();

        let signature = STANDARD.encode(signature);

        Ok(signature)
    }

    async fn verify(
        &self,
        transaction_hash_hex: &str,
        signature_asn1_base64: &str,
        key: &str,
    ) -> crate::Result<bool> {
        let kms_client = &self.sign;

        // Resolve key to key ID
        let key_metadata = self.describe_key_metadata(kms_client, key).await?;
        let key_id = key_metadata.key_id;

        // Decode the digest hex to bytes
        let digest_bytes = hex::decode(transaction_hash_hex).map_err(|e| {
            let msg = format!("Failed to decode transaction hash hex: {e}");
            error!("{}", msg);
            crate::KmsError::Msg(msg)
        })?;

        // Decode the signature base64 to bytes (ASN.1 DER format expected)
        let signature_bytes = STANDARD.decode(signature_asn1_base64).map_err(|e| {
            let msg = format!("Failed to decode signature base64: {e}");
            error!("{}", msg);
            crate::KmsError::Msg(msg)
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
                crate::KmsError::Msg(msg)
            })?;

        // The 'signature_valid' field indicates validity
        Ok(verify_output.signature_valid)
    }

    async fn delete_key(&self, key: &str) -> crate::Result<bool> {
        let Some(kms_client) = &self.delete else {
            tracing::warn!(
                "Attempted to delete key, but delete_kms client is not configured/enabled"
            );
            return Ok(false);
        };

        // Resolve key to key ID
        let key_metadata = self.describe_key_metadata(kms_client, key).await?;

        let key_id = key_metadata.key_id.clone();
        let alias_name = Self::format_alias(key);

        kms_client
            .delete_alias()
            .alias_name(&alias_name)
            .send()
            .await
            .map_err(|e| {
                let msg = format!("Failed to delete alias {alias_name}: {e:?}");
                error!("{}", msg);
                crate::KmsError::Msg(msg)
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
                crate::KmsError::Msg(msg)
            })?;

        Ok(true)
    }

    async fn list_keys(&self) -> crate::Result<Vec<KeyEntry>> {
        let Some(kms_client) = &self.list else {
            tracing::warn!("Attempted to list keys, but list_kms client is not configured/enabled");
            return Ok(vec![]);
        };

        let mut paginator = kms_client.list_aliases().into_paginator().send();
        let mut results = Vec::<KeyEntry>::new();

        while let Some(page_result) = paginator.next().await {
            let page = page_result.map_err(|e| {
                let msg = format!("Failed to list aliases: {e:?}");
                error!("{}", msg);
                crate::KmsError::Msg(msg)
            })?;

            let aliases = page.aliases.unwrap_or_default();

            for alias in aliases {
                let (Some(alias_name), Some(key_id)) = (alias.alias_name, alias.target_key_id)
                else {
                    continue;
                };

                if alias_name.starts_with("alias/aws/") {
                    continue;
                }

                let key_metadata = self.describe_key_metadata(kms_client, &alias_name).await?;

                let is_enabled =
                    key_metadata.key_state.as_ref() == Some(&aws_sdk_kms::types::KeyState::Enabled);

                if !is_enabled {
                    continue;
                }

                // Skip symmetric keys as they don't have public keys
                // Only process asymmetric keys (ECC, RSA, etc.)
                if key_metadata.key_spec.as_ref() == Some(&KeySpec::SymmetricDefault) {
                    continue;
                }

                let address = alias_name.trim_start_matches("alias/").to_string();
                let public_key_base64 = self.get_public_key_base64(kms_client, &key_id).await?;

                results.push(KeyEntry {
                    address: address.into(),
                    public_key_base64: public_key_base64.into(),
                    public_key: None.into(), // recomputed later per keys service list_keys
                    key_id: key_id.into(),
                });
            }
        }

        Ok(results)
    }

    /// Resolves an alias and fetches the base64-encoded public key associated with it.
    ///
    /// # Arguments
    /// * `kms_client` - The AWS KMS client used for the request.
    /// * `alias` - The alias name (with or without the `alias/` prefix).
    ///
    /// # Returns
    /// * `Ok(String)` - The base64-encoded public key.
    /// * `Err(String)` - If resolving the alias or fetching the key fails.
    async fn get_public_key(&self, alias: &str) -> crate::Result<String> {
        // Ensure alias is in the correct format
        let alias_name = if alias.starts_with("alias/") {
            alias.to_string()
        } else {
            format!("alias/{alias}")
        };

        let kms_client = &self.sign; // Sign credentials are used to get a public key

        let key_metadata = self.describe_key_metadata(kms_client, &alias_name).await?;
        let key_id = &key_metadata.key_id;

        self.get_public_key_base64(kms_client, key_id).await
    }
}

impl AWSKmsClientService {
    /// Retrieves the public key for the given key ID from AWS KMS.
    ///
    /// # Arguments
    /// * `kms_client` - A reference to the KMS client used to call `get_public_key`.
    /// * `key_id` - The ID of the key to retrieve the public key for.
    ///
    /// # Returns
    /// * `Ok(String)` - The base64-encoded public key.
    /// * `Err(String)` - Error message if the call fails or the key is missing.
    async fn get_public_key_base64(
        &self,
        kms_client: &aws_sdk_kms::Client,
        key_id: &str,
    ) -> crate::Result<String> {
        let pubkey_resp = kms_client
            .get_public_key()
            .key_id(key_id)
            .send()
            .await
            .map_err(|e| {
                let msg = format!("Could not get public key for key_id {key_id}: {e:?}");
                error!("{}", msg);
                crate::KmsError::Msg(msg)
            })?;

        let pubkey = pubkey_resp.public_key.ok_or_else(|| {
            let msg = format!("Missing public key for key_id {key_id}");
            error!("{}", msg);
            crate::KmsError::Msg(msg)
        })?;

        Ok(STANDARD.encode(pubkey.as_ref()))
    }

    /// Retrieves `KeyMetadata` for a given key key by calling AWS KMS `DescribeKey`.
    ///
    /// # Arguments
    /// * `key` - The key name without the `alias/` prefix.
    ///
    /// # Returns
    /// * `Ok(KeyMetadata)` if the key is found.
    /// * `Err(String)` if the request fails or metadata is missing.
    async fn describe_key_metadata(
        &self,
        kms_client: &aws_sdk_kms::Client,
        key: &str,
    ) -> crate::Result<KeyMetadata> {
        let alias_name = Self::format_alias(key);

        let output = kms_client
            .describe_key()
            .key_id(&alias_name)
            .send()
            .await
            .map_err(|e| {
                let msg = format!("Failed to describe key for alias {alias_name}: {e:?}");
                error!("{}", msg);
                crate::KmsError::Msg(msg)
            })?;

        output.key_metadata.ok_or_else(|| {
            error!("KeyMetadata not found for alias {alias_name}");
            crate::KmsError::AliasNotFound {
                alias: alias_name.clone(),
            }
        })
    }

    fn format_alias(alias: &str) -> String {
        if alias.starts_with("alias/") {
            alias.to_string()
        } else {
            format!("alias/{alias}")
        }
    }
}

async fn aws_config_to_sdk_config(
    region: Region,
    endpoint: String,
    creds: Credentials,
) -> SdkConfig {
    let config_loader = aws_config::defaults(aws_config::BehaviorVersion::latest());
    config_loader
        .region(region)
        .endpoint_url(endpoint)
        .credentials_provider(creds)
        .load()
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::{AwsConfig, AwsCreds},
        constants::{AWS_KMS_ENDPOINT_PATTERN, DEFAULT_AWS_REGION},
    };

    #[tokio::test]
    async fn test_aws_kms_client_service_new_success() {
        let dummy_creds = AwsCreds {
            access_key_id: "dummy_access_key".into(),
            secret_access_key: "dummy_secret_key".into(),
        };

        let aws_config = AwsConfig {
            region: DEFAULT_AWS_REGION.into(),
            endpoint: AWS_KMS_ENDPOINT_PATTERN.replace("{}", DEFAULT_AWS_REGION),
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
            region: DEFAULT_AWS_REGION.into(),
            endpoint: AWS_KMS_ENDPOINT_PATTERN.replace("{}", DEFAULT_AWS_REGION),
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
