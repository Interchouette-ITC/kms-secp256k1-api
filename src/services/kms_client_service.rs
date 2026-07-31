use crate::services::keys_service::KeyEntry;

#[async_trait::async_trait]
pub trait KmsClientService: Send + Sync {
    async fn create_key(&self) -> crate::Result<(String, String)>;
    async fn create_alias(&self, key_id: &str, alias: &str) -> crate::Result<()>;
    async fn sign(&self, transaction_hash_hex: &str, alias: &str) -> crate::Result<String>;
    async fn verify(
        &self,
        transaction_hash_hex: &str,
        signature_with_prefix: &str,
        alias: &str,
    ) -> crate::Result<bool>;
    async fn delete_key(&self, alias: &str) -> crate::Result<bool>;
    async fn list_keys(&self) -> crate::Result<Vec<KeyEntry>>;
    async fn get_public_key(&self, alias: &str) -> crate::Result<String>;
}
