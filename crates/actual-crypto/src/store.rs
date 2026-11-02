use std::collections::HashMap;

use crate::cipher::{decrypt, encrypt, EncryptedData};
use crate::error::CryptoError;
use crate::key::Key;

/// Manages active encryption keys in memory, matching upstream Actual Budget's key registry.
#[derive(Debug, Default, Clone)]
pub struct KeyStore {
    keys: HashMap<String, Key>,
}

impl KeyStore {
    /// Create a new empty `KeyStore`.
    pub fn new() -> Self {
        Self {
            keys: HashMap::new(),
        }
    }

    /// Load a key into the registry.
    pub fn load_key(&mut self, key: Key) {
        self.keys.insert(key.id().to_string(), key);
    }

    /// Unload a key by its ID.
    pub fn unload_key(&mut self, key_id: &str) {
        self.keys.remove(key_id);
    }

    /// Unload all keys from the registry.
    pub fn unload_all_keys(&mut self) {
        self.keys.clear();
    }

    /// Retrieve a reference to a key by its ID.
    pub fn get_key(&self, key_id: &str) -> Result<&Key, CryptoError> {
        self.keys
            .get(key_id)
            .ok_or_else(|| CryptoError::KeyNotFound(key_id.to_string()))
    }

    /// Check if a key is loaded.
    pub fn has_key(&self, key_id: &str) -> bool {
        self.keys.contains_key(key_id)
    }

    /// Encrypt a plaintext byte slice using the specified key ID.
    pub fn encrypt(&self, key_id: &str, plaintext: &[u8]) -> Result<EncryptedData, CryptoError> {
        let key = self.get_key(key_id)?;
        encrypt(key, plaintext)
    }

    /// Decrypt an `EncryptedData` payload using the key specified in its metadata.
    pub fn decrypt(&self, encrypted: &EncryptedData) -> Result<Vec<u8>, CryptoError> {
        let key = self.get_key(&encrypted.meta.key_id)?;
        decrypt(key, encrypted)
    }
}
