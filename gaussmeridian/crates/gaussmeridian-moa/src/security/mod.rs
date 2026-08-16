use std::path::Path;
use aes_gcm::{
    aead::{Aead, KeyInit, AeadCore},
    Aes256Gcm, Key, Nonce,
};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use tracing::error;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SecurityError {
    #[error("Encryption error: {0}")]
    Encryption(String),
    #[error("Decryption error: {0}")]
    Decryption(String),
    #[error("Key management error: {0}")]
    KeyManagement(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Base64 error: {0}")]
    Base64(#[from] base64::DecodeError),
    #[error("Password hash error: {0}")]
    PasswordHash(String),
}

pub struct EncryptedData {
    pub ciphertext: Vec<u8>,
    pub nonce: Vec<u8>,
}

#[derive(Debug)]
pub struct KeyManager {
    key: Key<Aes256Gcm>,
}

impl KeyManager {
    pub fn new(key_path: impl AsRef<Path>) -> Result<Self, SecurityError> {
        let key_bytes = std::fs::read(key_path)
            .map_err(|e| SecurityError::KeyManagement(format!("Failed to read key file: {}", e)))?;

        if key_bytes.len() != 32 { // AES-256 key must be 32 bytes
            return Err(SecurityError::KeyManagement("Invalid key length: must be 32 bytes".to_string()));
        }
        // from_slice will panic if length is incorrect, but we checked.
        let key_ref = Key::<Aes256Gcm>::from_slice(&key_bytes);
        let key_owned = key_ref.clone(); // Clone to own the key for the struct

        Ok(Self { key: key_owned })
    }

    pub fn encrypt(&self, data: &[u8]) -> Result<EncryptedData, SecurityError> {
        let cipher = Aes256Gcm::new(&self.key);
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let ciphertext = cipher
            .encrypt(&nonce, data)
            .map_err(|_| SecurityError::Encryption("Encryption failed".to_string()))?;

        Ok(EncryptedData {
            ciphertext,
            nonce: nonce.to_vec(),
        })
    }

    pub fn decrypt(&self, encrypted: &EncryptedData) -> Result<Vec<u8>, SecurityError> {
        let cipher = Aes256Gcm::new(&self.key);
        let nonce = Nonce::from_slice(&encrypted.nonce);
        cipher
            .decrypt(nonce, encrypted.ciphertext.as_slice())
            .map_err(|_| SecurityError::Decryption("Decryption failed".to_string()))
    }

    pub fn hash_password(&self, password: &str) -> Result<String, SecurityError> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();

        argon2
            .hash_password(password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|e| SecurityError::PasswordHash(e.to_string()))
    }

    pub fn verify_password(&self, password: &str, hash: &str) -> Result<bool, SecurityError> {
        let parsed_hash = PasswordHash::new(hash)
            .map_err(|e| SecurityError::PasswordHash(e.to_string()))?;

        Ok(Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_encryption_decryption() -> Result<(), SecurityError> {
        let key = Aes256Gcm::generate_key(&mut OsRng);
        let temp_file = NamedTempFile::new().unwrap();
        std::fs::write(temp_file.path(), key.as_slice()).unwrap();

        let key_manager = KeyManager::new(temp_file.path())?;
        let data = b"Hello, World!";

        let encrypted = key_manager.encrypt(data)?;
        let decrypted = key_manager.decrypt(&encrypted)?;

        assert_eq!(data.as_slice(), decrypted.as_slice());
        Ok(())
    }

    #[test]
    fn test_password_hashing() -> Result<(), SecurityError> {
        let key = Aes256Gcm::generate_key(&mut OsRng);
        let temp_file = NamedTempFile::new().unwrap();
        std::fs::write(temp_file.path(), key.as_slice()).unwrap();

        let key_manager = KeyManager::new(temp_file.path())?;
        let password = "secret123";

        let hash = key_manager.hash_password(password)?;
        assert!(key_manager.verify_password(password, &hash)?);
        assert!(!key_manager.verify_password("wrong", &hash)?);

        Ok(())
    }
} 