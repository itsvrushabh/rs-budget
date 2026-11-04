//! CRDT dataset messages, envelope encoding, and authenticated encryption wrappers.

use crate::error::SyncError;
use crate::proto::{EncryptedData as ProtoEncryptedData, Message, MessageEnvelope};
use crate::timestamp::Timestamp;
use actual_crypto::{cipher, EncryptedData, EncryptionMeta, Key, ENCRYPTION_ALGORITHM};
use base64::prelude::*;
use prost::Message as ProstMessage;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// A single atomic mutation within the CRDT dataset.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrdtMessage {
    /// Hybrid Logical Clock timestamp assigned to this change.
    pub timestamp: Timestamp,
    /// Target dataset or table (e.g. "transactions", "categories", "accounts").
    pub dataset: String,
    /// Unique primary key / row identifier.
    pub row: String,
    /// Modified attribute / column name (e.g. "amount", "name", "notes").
    pub column: String,
    /// String-serialized attribute value.
    pub value: String,
}

impl CrdtMessage {
    /// Constructs a new `CrdtMessage`.
    #[must_use]
    pub fn new(
        timestamp: Timestamp,
        dataset: impl Into<String>,
        row: impl Into<String>,
        column: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        Self {
            timestamp,
            dataset: dataset.into(),
            row: row.into(),
            column: column.into(),
            value: value.into(),
        }
    }

    /// Encodes this CRDT message into a protobuf `MessageEnvelope`, optionally encrypting the payload.
    pub fn to_envelope(&self, encryption_key: Option<&Key>) -> Result<MessageEnvelope, SyncError> {
        let inner_msg = Message {
            dataset: self.dataset.clone(),
            row: self.row.clone(),
            column: self.column.clone(),
            value: self.value.clone(),
        };

        let mut raw_bytes = Vec::with_capacity(inner_msg.encoded_len());
        inner_msg.encode(&mut raw_bytes)?;

        let (is_encrypted, content) = if let Some(key) = encryption_key {
            let enc_payload = cipher::encrypt(key, &raw_bytes)?;
            let iv = BASE64_STANDARD
                .decode(enc_payload.meta.iv.trim())
                .map_err(|e| SyncError::Crypto(actual_crypto::CryptoError::Base64(e)))?;
            let auth_tag = BASE64_STANDARD
                .decode(enc_payload.meta.auth_tag.trim())
                .map_err(|e| SyncError::Crypto(actual_crypto::CryptoError::Base64(e)))?;

            let proto_enc = ProtoEncryptedData {
                iv,
                auth_tag,
                data: enc_payload.value,
            };
            let mut enc_bytes = Vec::with_capacity(proto_enc.encoded_len());
            proto_enc.encode(&mut enc_bytes)?;
            (true, enc_bytes)
        } else {
            (false, raw_bytes)
        };

        Ok(MessageEnvelope {
            timestamp: self.timestamp.to_string(),
            is_encrypted,
            content,
        })
    }

    /// Decodes a `CrdtMessage` from a protobuf `MessageEnvelope`, decrypting if necessary.
    pub fn from_envelope(
        envelope: &MessageEnvelope,
        encryption_key: Option<&Key>,
    ) -> Result<Self, SyncError> {
        let timestamp = Timestamp::from_str(&envelope.timestamp)?;

        let raw_bytes = if envelope.is_encrypted {
            let key = encryption_key.ok_or_else(|| {
                SyncError::Crypto(actual_crypto::CryptoError::KeyNotFound(
                    "Encryption key required to decrypt sync message envelope".into(),
                ))
            })?;

            let enc_proto = ProtoEncryptedData::decode(envelope.content.as_slice())?;
            let enc_data = EncryptedData {
                value: enc_proto.data,
                meta: EncryptionMeta {
                    key_id: key.id().to_string(),
                    algorithm: ENCRYPTION_ALGORITHM.to_string(),
                    iv: BASE64_STANDARD.encode(&enc_proto.iv),
                    auth_tag: BASE64_STANDARD.encode(&enc_proto.auth_tag),
                },
            };

            cipher::decrypt(key, &enc_data)?
        } else {
            envelope.content.clone()
        };

        let inner_msg = Message::decode(raw_bytes.as_slice())?;

        Ok(Self {
            timestamp,
            dataset: inner_msg.dataset,
            row: inner_msg.row,
            column: inner_msg.column,
            value: inner_msg.value,
        })
    }
}
