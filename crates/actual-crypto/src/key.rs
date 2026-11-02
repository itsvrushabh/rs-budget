use base64::prelude::*;
use pbkdf2::pbkdf2_hmac;
use serde::{Deserialize, Serialize};
use sha2::Sha512;
use uuid::Uuid;

use crate::error::CryptoError;

pub const PBKDF2_ITERATIONS: u32 = 10_000;
pub const KEY_LENGTH_BYTES: usize = 32;

/// A 256-bit cryptographic master encryption key compatible with Actual Budget.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Key {
    id: String,
    raw: [u8; KEY_LENGTH_BYTES],
    base64: String,
}

impl Key {
    /// Construct a `Key` from a 32-byte raw key array and an optional ID.
    pub fn new(id: Option<String>, raw: [u8; KEY_LENGTH_BYTES]) -> Self {
        let id = id.unwrap_or_else(|| Uuid::new_v4().to_string());
        let base64 = BASE64_STANDARD.encode(raw);
        Self { id, raw, base64 }
    }

    /// Derive a 256-bit key from a password and salt using PBKDF2 (HMAC-SHA-512, 10,000 iterations).
    pub fn from_password(
        id: Option<String>,
        password: &str,
        salt: &str,
    ) -> Result<Self, CryptoError> {
        let mut raw = [0u8; KEY_LENGTH_BYTES];
        pbkdf2_hmac::<Sha512>(
            password.as_bytes(),
            salt.as_bytes(),
            PBKDF2_ITERATIONS,
            &mut raw,
        );
        Ok(Self::new(id, raw))
    }

    /// Import a key from a Base64-encoded 32-byte string.
    pub fn from_base64(id: Option<String>, base64_str: &str) -> Result<Self, CryptoError> {
        let bytes = BASE64_STANDARD.decode(base64_str.trim())?;
        if bytes.len() != KEY_LENGTH_BYTES {
            return Err(CryptoError::InvalidKeyLength {
                expected: KEY_LENGTH_BYTES,
                got: bytes.len(),
            });
        }
        let mut raw = [0u8; KEY_LENGTH_BYTES];
        raw.copy_from_slice(&bytes);
        let id = id.unwrap_or_else(|| Uuid::new_v4().to_string());
        Ok(Self {
            id,
            raw,
            base64: base64_str.trim().to_string(),
        })
    }

    /// Return the key's unique identifier.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Return a reference to the raw 32-byte key material.
    pub fn raw(&self) -> &[u8; KEY_LENGTH_BYTES] {
        &self.raw
    }

    /// Return the Base64 representation of the key.
    pub fn base64(&self) -> &str {
        &self.base64
    }

    /// Serialize the key to the upstream JSON format: `{"id": "...", "base64": "..."}`.
    pub fn serialize(&self) -> KeySerialized {
        KeySerialized {
            id: self.id.clone(),
            base64: self.base64.clone(),
        }
    }
}

/// JSON-serializable key representation matching upstream Actual Budget.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KeySerialized {
    pub id: String,
    pub base64: String,
}
