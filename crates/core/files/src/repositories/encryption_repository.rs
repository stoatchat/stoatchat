use anyhow::Result;

pub trait EncryptionRepository: Send + Sync + 'static {
    fn decrypt_buffer(&self, buf: Vec<u8>, iv: &str) -> anyhow::Result<Vec<u8>>;
    fn encrypt_buffer(&self, buf: &[u8]) -> Result<(Vec<u8>, String)>;
}
