//! `actual-crypto` implements PBKDF2 key derivation and AES-256-GCM
//! authenticated encryption compatible with Actual Budget.

pub mod cipher;
pub mod error;
pub mod key;
pub mod store;

pub use cipher::{
    decrypt, encrypt, encrypt_with_iv, random_bytes, EncryptedData, EncryptionMeta,
    AUTH_TAG_LENGTH_BYTES, ENCRYPTION_ALGORITHM, IV_LENGTH_BYTES,
};
pub use error::CryptoError;
pub use key::{Key, KeySerialized, KEY_LENGTH_BYTES, PBKDF2_ITERATIONS};
pub use store::KeyStore;
