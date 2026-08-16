//! BYOK (Bring Your Own Key) vault — AES-256-GCM encrypted provider key storage.
//!
//! Provider API keys are encrypted with a master key derived from `BYOK_MASTER_KEY`
//! (a 32-byte, base64-encoded secret in `.env`). Plaintext keys are never written to
//! logs, files, or the database.
//!
//! # Storage layout
//! Each `api_key` record in SurrealDB holds a `byok_keys` JSON object:
//!   `{ "openai": "<base64(nonce || ciphertext)>", "anthropic": "...", ... }`
//!
//! # Security invariants
//! - `BYOK_MASTER_KEY` must be exactly 32 bytes after base64 decoding.
//! - A fresh 96-bit random nonce is generated for every encrypt call.
//! - The nonce is prepended to the ciphertext before base64 encoding.
//! - Plaintext keys are held in `secrecy::SecretString` and zeroed on drop.

use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    AeadCore, Aes256Gcm, Key, Nonce,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use secrecy::{ExposeSecret, SecretString};
use thiserror::Error;

/// Errors from the BYOK vault
#[derive(Debug, Error)]
pub enum ByokError {
    #[error("BYOK_MASTER_KEY not set or invalid: {0}")]
    MasterKeyInvalid(String),

    #[error("encryption failed: {0}")]
    EncryptionFailed(String),

    #[error("decryption failed: {0}")]
    DecryptionFailed(String),

    #[error("invalid ciphertext encoding: {0}")]
    InvalidCiphertext(String),
}

/// Vault for encrypting and decrypting per-provider API keys.
///
/// One `ByokVault` is created at server startup and held in `AppState`.
#[derive(Clone)]
pub struct ByokVault {
    cipher: Aes256Gcm,
}

impl ByokVault {
    /// Create a new vault using `BYOK_MASTER_KEY` from the environment.
    ///
    /// The env var must be a standard-alphabet base64-encoded 32-byte key.
    pub fn from_env() -> Result<Self, ByokError> {
        let raw = std::env::var("BYOK_MASTER_KEY").map_err(|_| {
            ByokError::MasterKeyInvalid(
                "BYOK_MASTER_KEY is not set; BYOK functionality is disabled".to_string(),
            )
        })?;

        let bytes = BASE64.decode(raw.trim()).map_err(|e| {
            ByokError::MasterKeyInvalid(format!("base64 decode failed: {}", e))
        })?;

        if bytes.len() != 32 {
            return Err(ByokError::MasterKeyInvalid(format!(
                "key must be 32 bytes after decoding, got {}",
                bytes.len()
            )));
        }

        let key = Key::<Aes256Gcm>::from_slice(&bytes);
        let cipher = Aes256Gcm::new(key);

        Ok(Self { cipher })
    }

    /// Create a vault directly from a 32-byte key slice.
    ///
    /// Prefer `from_env` in production. This constructor exists for testing
    /// to avoid the env-var race condition in parallel test threads.
    #[cfg(test)]
    pub(crate) fn with_key(key: &[u8; 32]) -> Self {
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
        Self { cipher }
    }

    /// Encrypt a plaintext provider API key.
    ///
    /// Returns `"<base64(12-byte nonce || ciphertext)>"`.
    /// Never logs or surfaces the plaintext key.
    pub fn encrypt(&self, plaintext: &SecretString) -> Result<String, ByokError> {
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let ciphertext = self
            .cipher
            .encrypt(&nonce, plaintext.expose_secret().as_bytes())
            .map_err(|e| ByokError::EncryptionFailed(e.to_string()))?;

        // Prepend 12-byte nonce to ciphertext, then base64-encode the whole blob
        let mut blob = Vec::with_capacity(12 + ciphertext.len());
        blob.extend_from_slice(nonce.as_slice());
        blob.extend_from_slice(&ciphertext);

        Ok(BASE64.encode(blob))
    }

    /// Decrypt a stored provider API key blob.
    ///
    /// `blob` must be the output of `encrypt`. Returns a `SecretString`; the caller
    /// must call `expose_secret()` at the last possible moment and discard immediately.
    pub fn decrypt(&self, blob: &str) -> Result<SecretString, ByokError> {
        let raw = BASE64
            .decode(blob.trim())
            .map_err(|e| ByokError::InvalidCiphertext(format!("base64: {}", e)))?;

        if raw.len() < 12 {
            return Err(ByokError::InvalidCiphertext(
                "blob too short (must be at least 12 bytes)".to_string(),
            ));
        }

        let nonce = Nonce::from_slice(&raw[..12]);
        let ciphertext = &raw[12..];

        let plaintext_bytes = self
            .cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| ByokError::DecryptionFailed(e.to_string()))?;

        let plaintext = String::from_utf8(plaintext_bytes).map_err(|e| {
            ByokError::DecryptionFailed(format!("utf-8 decode failed: {}", e))
        })?;

        Ok(SecretString::new(plaintext.into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_vault() -> ByokVault {
        ByokVault::with_key(&[42u8; 32])
    }

    #[test]
    fn test_roundtrip() {
        let vault = test_vault();
        let plaintext = SecretString::new("sk-test-1234567890abcdef".into());
        let encrypted = vault.encrypt(&plaintext).unwrap();
        let decrypted = vault.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted.expose_secret(), plaintext.expose_secret());
    }

    #[test]
    fn test_different_nonces() {
        let vault = test_vault();
        let plaintext = SecretString::new("same-key".into());
        let enc1 = vault.encrypt(&plaintext).unwrap();
        let enc2 = vault.encrypt(&plaintext).unwrap();
        // Different nonces → different ciphertext blobs
        assert_ne!(enc1, enc2);
        // But both decrypt to the same value
        assert_eq!(
            vault.decrypt(&enc1).unwrap().expose_secret(),
            vault.decrypt(&enc2).unwrap().expose_secret()
        );
    }

    #[test]
    fn test_tampered_ciphertext_fails() {
        let vault = test_vault();
        let plaintext = SecretString::new("secret-key".into());
        let encrypted = vault.encrypt(&plaintext).unwrap();
        // Corrupt a byte in the middle of the blob
        let mut raw = BASE64.decode(&encrypted).unwrap();
        raw[14] ^= 0xFF;
        let tampered = BASE64.encode(raw);
        assert!(vault.decrypt(&tampered).is_err());
    }

    #[test]
    fn test_key_length_validation() {
        // A 16-byte key is not valid — from_env must reject it
        let raw_b64 = BASE64.encode([0u8; 16]);
        // Temporarily set BYOK_MASTER_KEY to a short key (test thread only)
        let prev = std::env::var("BYOK_MASTER_KEY").ok();
        std::env::set_var("BYOK_MASTER_KEY", &raw_b64);
        assert!(ByokVault::from_env().is_err());
        // Restore previous value to avoid polluting other tests
        match prev {
            Some(v) => std::env::set_var("BYOK_MASTER_KEY", v),
            None => std::env::remove_var("BYOK_MASTER_KEY"),
        }
    }
}
