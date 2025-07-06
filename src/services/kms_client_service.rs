#[async_trait::async_trait]
pub trait KmsClientService: Send + Sync {
    async fn create_key(&self) -> Result<(String, String), String>;
    async fn create_alias(&self, key_id: &str, public_key: &str) -> Result<(), String>;
    async fn sign(&self, transaction_hash_hex: &str, public_key: &str) -> Result<String, String>;
    async fn verify(
        &self,
        transaction_hash_hex: &str,
        signature_with_prefix: &str,
        public_key: &str,
    ) -> Result<bool, String>;
    async fn delete_key(&self, public_key: &str) -> Result<bool, String>;
    async fn list_keys(&self) -> Result<Vec<(String, String)>, String>;
}
