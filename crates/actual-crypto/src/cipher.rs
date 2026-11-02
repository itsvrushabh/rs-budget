use aes_gcm::aead::rand_core::RngCore;
use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::prelude::*;
use serde::{Deserialize, Serialize};

use crate::error::CryptoError;
use crate::key::Key;

pub const ENCRYPTION_ALGORITHM: &str = "aes-256-gcm";
pub const IV_LENGTH_BYTES: usize = 12;
pub const AUTH_TAG_LENGTH_BYTES: usize = 16;

/// Encrypted payload with stripped auth tag and associated metadata matching upstream Actual Budget.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncryptedData {
    #[serde(with = "base64_serde")]
    pub value: Vec<u8>,
    pub meta: EncryptionMeta,
}

/// Metadata accompanying encrypted data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EncryptionMeta {
    pub key_id: String,
    pub algorithm: String,
    pub iv: String,
    pub auth_tag: String,
}

mod base64_serde {
    use base64::prelude::*;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&BASE64_STANDARD.encode(bytes))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        BASE64_STANDARD
            .decode(s.trim())
            .map_err(serde::de::Error::custom)
    }
}

/// Generate cryptographically secure random bytes.
pub fn random_bytes(n: usize) -> Vec<u8> {
    let mut bytes = vec![0u8; n];
    OsRng.fill_bytes(&mut bytes);
    bytes
}

/// Encrypt a plaintext byte slice using AES-256-GCM with a randomly generated 12-byte IV.
pub fn encrypt(key: &Key, plaintext: &[u8]) -> Result<EncryptedData, CryptoError> {
    let mut iv = [0u8; IV_LENGTH_BYTES];
    OsRng.fill_bytes(&mut iv);
    encrypt_with_iv(key, plaintext, &iv)
}

/// Encrypt a plaintext byte slice using AES-256-GCM with a specified 12-byte IV.
pub fn encrypt_with_iv(
    key: &Key,
    plaintext: &[u8],
    iv: &[u8; IV_LENGTH_BYTES],
) -> Result<EncryptedData, CryptoError> {
    let cipher =
        Aes256Gcm::new_from_slice(key.raw()).map_err(|e| CryptoError::Encryption(e.to_string()))?;
    let nonce = Nonce::from_slice(iv);

    let ciphertext_with_tag = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| CryptoError::Encryption(e.to_string()))?;

    if ciphertext_with_tag.len() < AUTH_TAG_LENGTH_BYTES {
        return Err(CryptoError::Encryption(
            "Ciphertext shorter than authentication tag".to_string(),
        ));
    }

    let tag_start = ciphertext_with_tag.len() - AUTH_TAG_LENGTH_BYTES;
    let value = ciphertext_with_tag[..tag_start].to_vec();
    let auth_tag = &ciphertext_with_tag[tag_start..];

    Ok(EncryptedData {
        value,
        meta: EncryptionMeta {
            key_id: key.id().to_string(),
            algorithm: ENCRYPTION_ALGORITHM.to_string(),
            iv: BASE64_STANDARD.encode(iv),
            auth_tag: BASE64_STANDARD.encode(auth_tag),
        },
    })
}

/// Decrypt an `EncryptedData` payload using AES-256-GCM and verify its authentication tag.
pub fn decrypt(key: &Key, encrypted: &EncryptedData) -> Result<Vec<u8>, CryptoError> {
    if encrypted.meta.algorithm != ENCRYPTION_ALGORITHM {
        return Err(CryptoError::UnsupportedAlgorithm(
            encrypted.meta.algorithm.clone(),
        ));
    }

    let iv_bytes = BASE64_STANDARD.decode(encrypted.meta.iv.trim())?;
    if iv_bytes.len() != IV_LENGTH_BYTES {
        return Err(CryptoError::InvalidIvLength {
            expected: IV_LENGTH_BYTES,
            got: iv_bytes.len(),
        });
    }

    let tag_bytes = BASE64_STANDARD.decode(encrypted.meta.auth_tag.trim())?;
    if tag_bytes.len() != AUTH_TAG_LENGTH_BYTES {
        return Err(CryptoError::InvalidAuthTagLength {
            expected: AUTH_TAG_LENGTH_BYTES,
            got: tag_bytes.len(),
        });
    }

    let mut combined = Vec::with_capacity(encrypted.value.len() + tag_bytes.len());
    combined.extend_from_slice(&encrypted.value);
    combined.extend_from_slice(&tag_bytes);

    let cipher =
        Aes256Gcm::new_from_slice(key.raw()).map_err(|e| CryptoError::Decryption(e.to_string()))?;
    let nonce = Nonce::from_slice(&iv_bytes);

    let plaintext = cipher
        .decrypt(nonce, combined.as_ref())
        .map_err(|e| CryptoError::Decryption(e.to_string()))?;

    Ok(plaintext)
}
