use aes_gcm::{
    aead::{Aead, AeadCore, AeadMutInPlace, OsRng},
    Aes256Gcm, Key, KeyInit, Nonce,
};
use base64::{prelude::BASE64_STANDARD, Engine};

use crate::EncryptionRepository;

pub struct EncryptionKey {
    key: String,
}

impl EncryptionKey {
    pub async fn from_config() -> EncryptionKey {
        EncryptionKey::new(revolt_config::config().await.files.encryption_key)
    }

    pub fn new(key: String) -> EncryptionKey {
        EncryptionKey { key }
    }

    fn create_cipher(&self) -> Aes256Gcm {
        let key = &BASE64_STANDARD
            .decode(self.key.clone())
            .expect("valid base64 string")[..];
        let key: &Key<Aes256Gcm> = key.into();
        Aes256Gcm::new(key)
    }
}

impl EncryptionRepository for EncryptionKey {
    fn decrypt_buffer(&self, mut buf: Vec<u8>, iv: &str) -> anyhow::Result<Vec<u8>> {
        let iv = &BASE64_STANDARD.decode(iv).unwrap()[..];
        let iv: &Nonce<typenum::consts::U12> = iv.into();

        self.create_cipher()
            .decrypt_in_place(iv, b"", &mut buf)
            .map_err(|error| {
                tracing::error!("{}", error);
                anyhow::anyhow!("EncryptionRepository: decryption failed")
            })?;

        Ok(buf)
    }

    fn encrypt_buffer(&self, buf: &[u8]) -> anyhow::Result<(Vec<u8>, String)> {
        let iv = Aes256Gcm::generate_nonce(&mut OsRng);

        let buf = self.create_cipher().encrypt(&iv, buf).map_err(|error| {
            tracing::error!("{}", error);
            anyhow::anyhow!("EncryptionRepository: encryption failed")
        })?;

        Ok((buf, BASE64_STANDARD.encode(iv)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_and_decrypt() {
        let encryption =
            EncryptionKey::new("XkbJ8gBzrouQ+15Ri23xCC81+aZE26Z6+gXzglFxOD4=".to_string());

        let buf: Vec<u8> = vec![67];
        let (ciphertext, iv) = encryption.encrypt_buffer(&buf[..]).unwrap();
        assert_eq!(ciphertext.len(), 17);

        let plaintext = encryption.decrypt_buffer(ciphertext, &iv).unwrap();
        assert_eq!(plaintext.len(), 1);
        assert_eq!(plaintext[0], 67);
    }
}
