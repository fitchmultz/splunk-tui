//! Encryption utilities for configuration at rest.
//!
//! Responsibilities:
//! - Provide AES-256-GCM encryption and decryption.
//! - Handle key derivation using Argon2id.
//! - Manage master key sources (Keyring, Password, Env).
//!
//! Does NOT handle:
//! - Config file persistence (handled by profiles.rs).
//! - Keyring storage (handled by profiles.rs or specifically for encryption key).

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use argon2::Argon2;
use rand::Rng;
use secrecy::{ExposeSecret, SecretString};
use thiserror::Error;

/// Errors that can occur during encryption operations.
#[derive(Debug, Error)]
pub enum EncryptionError {
    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),

    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),

    #[error("Key derivation failed: {0}")]
    KeyDerivationFailed(String),

    #[error("Invalid key size: expected 32 bytes")]
    InvalidKeySize,

    #[error("Invalid nonce size: expected 12 bytes")]
    InvalidNonceSize,

    #[error("Keyring error: {0}")]
    KeyringError(#[from] keyring::Error),

    #[error("Environment variable error: {0}")]
    EnvError(String),
}

pub type Result<T> = std::result::Result<T, EncryptionError>;

/// Sources for the master encryption key.
#[derive(Debug, Clone)]
pub enum MasterKeySource {
    /// Use a random key stored in the OS keyring.
    Keyring,
    /// Derive a key from a user-provided password.
    Password(SecretString),
    /// Use a key provided in an environment variable.
    Env(String),
}

impl MasterKeySource {
    /// Resolves the master key source into a 32-byte key.
    pub fn resolve(&self, salt: Option<&[u8]>) -> Result<[u8; 32]> {
        match self {
            Self::Keyring => {
                let entry =
                    keyring::Entry::new(crate::types::KEYRING_SERVICE, "encryption-master-key")?;
                match entry.get_password() {
                    Ok(p) => {
                        let bytes = hex::decode(p)
                            .map_err(|e| EncryptionError::DecryptionFailed(e.to_string()))?;
                        if bytes.len() != 32 {
                            return Err(EncryptionError::InvalidKeySize);
                        }
                        let mut key = [0u8; 32];
                        key.copy_from_slice(&bytes);
                        Ok(key)
                    }
                    Err(keyring::Error::NoEntry) => {
                        // Generate a new random key and store it
                        let mut key = [0u8; 32];
                        rand::rng().fill(&mut key);
                        entry.set_password(&hex::encode(key))?;
                        Ok(key)
                    }
                    Err(e) => Err(e.into()),
                }
            }
            Self::Password(pw) => {
                let salt = salt.ok_or_else(|| {
                    EncryptionError::KeyDerivationFailed(
                        "Salt required for password-based encryption".to_string(),
                    )
                })?;
                Encryptor::derive_key(pw, salt)
            }
            Self::Env(var_name) => {
                let val = std::env::var(var_name).map_err(|_| {
                    EncryptionError::EnvError(format!("Environment variable {} not set", var_name))
                })?;
                let bytes = hex::decode(val)
                    .map_err(|e| EncryptionError::DecryptionFailed(e.to_string()))?;
                if bytes.len() != 32 {
                    return Err(EncryptionError::InvalidKeySize);
                }
                let mut key = [0u8; 32];
                key.copy_from_slice(&bytes);
                Ok(key)
            }
        }
    }
}

/// Core cryptographic logic for AES-256-GCM.
pub struct Encryptor;

impl Encryptor {
    /// Encrypts data using AES-256-GCM.
    /// Returns (ciphertext + tag, nonce).
    pub fn encrypt(data: &[u8], key: &[u8; 32]) -> Result<(Vec<u8>, [u8; 12])> {
        let cipher = Aes256Gcm::new(key.into());
        let mut nonce_bytes = [0u8; 12];
        rand::rng().fill(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, data)
            .map_err(|e| EncryptionError::EncryptionFailed(e.to_string()))?;

        Ok((ciphertext, nonce_bytes))
    }

    /// Decrypts data using AES-256-GCM.
    pub fn decrypt(ciphertext: &[u8], key: &[u8; 32], nonce: &[u8; 12]) -> Result<Vec<u8>> {
        let cipher = Aes256Gcm::new(key.into());
        let nonce = Nonce::from_slice(nonce);

        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| EncryptionError::DecryptionFailed(e.to_string()))?;

        Ok(plaintext)
    }

    /// Derives a 32-byte key from a password and salt using Argon2id.
    pub fn derive_key(password: &SecretString, salt: &[u8]) -> Result<[u8; 32]> {
        let argon2 = Argon2::default();
        let mut key = [0u8; 32];
        argon2
            .hash_password_into(password.expose_secret().as_bytes(), salt, &mut key)
            .map_err(|e| EncryptionError::KeyDerivationFailed(e.to_string()))?;
        Ok(key)
    }

    /// Generates a random 16-byte salt for key derivation.
    pub fn generate_salt() -> [u8; 16] {
        let mut salt = [0u8; 16];
        rand::rng().fill(&mut salt);
        salt
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::SecretString;

    #[test]
    fn test_encryption_roundtrip() {
        let key = [42u8; 32];
        let data = b"sensitive data";

        let (ciphertext, nonce) = Encryptor::encrypt(data, &key).unwrap();
        let decrypted = Encryptor::decrypt(&ciphertext, &key, &nonce).unwrap();

        assert_eq!(data, decrypted.as_slice());
    }

    #[test]
    fn test_key_derivation() {
        let password = SecretString::new("password".to_string().into());
        let salt = Encryptor::generate_salt();

        let key1 = Encryptor::derive_key(&password, &salt).unwrap();
        let key2 = Encryptor::derive_key(&password, &salt).unwrap();

        assert_eq!(key1, key2);

        let salt2 = Encryptor::generate_salt();
        let key3 = Encryptor::derive_key(&password, &salt2).unwrap();
        assert_ne!(key1, key3);
    }
}
