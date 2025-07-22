use crate::services::keys_service::KeyEntry;

#[async_trait::async_trait]
pub trait KmsClientService: Send + Sync {
    async fn create_key(&self) -> Result<(String, String), String>;
    async fn create_alias(&self, key_id: &str, alias: &str) -> Result<(), String>;
    async fn sign(&self, transaction_hash_hex: &str, alias: &str) -> Result<String, String>;
    async fn verify(
        &self,
        transaction_hash_hex: &str,
        signature_with_prefix: &str,
        alias: &str,
    ) -> Result<bool, String>;
    async fn delete_key(&self, alias: &str) -> Result<bool, String>;
    async fn list_keys(&self) -> Result<Vec<KeyEntry>, String>;
    async fn get_public_key(&self, alias: &str) -> Result<String, String>;
}
