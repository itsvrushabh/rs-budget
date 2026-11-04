//! `actual-crypto` implements PBKDF2 key derivation, AES-256-GCM authenticated encryption,
//! `.actual` archive packaging, backup zip compression, and master key rotation compatible with Actual Budget.

pub mod archive;
pub mod cipher;
pub mod error;
pub mod key;
pub mod rotation;
pub mod store;

pub use archive::{BudgetArchive, BudgetMetadata};
pub use cipher::{
    decrypt, encrypt, encrypt_with_iv, random_bytes, EncryptedData, EncryptionMeta,
    AUTH_TAG_LENGTH_BYTES, ENCRYPTION_ALGORITHM, IV_LENGTH_BYTES,
};
pub use error::CryptoError;
pub use key::{Key, KeySerialized, KEY_LENGTH_BYTES, PBKDF2_ITERATIONS};
pub use rotation::KeyRotator;
pub use store::KeyStore;
