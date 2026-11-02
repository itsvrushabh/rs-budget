//! Generated Protobuf codecs for sync wire format (`specs/proto/sync.proto`).

use prost::Message as ProstMessage;

#[allow(clippy::all)]
pub mod sync {
    include!(concat!(env!("OUT_DIR"), "/actual.sync.rs"));
}

pub use sync::{EncryptedData, Message, MessageEnvelope, SyncRequest, SyncResponse};

impl Message {
    /// Encodes this message to a protobuf byte vector.
    #[must_use]
    pub fn encode_to_vec(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(self.encoded_len());
        self.encode(&mut buf)
            .expect("encoding into Vec never fails");
        buf
    }

    /// Decodes a `Message` from a byte slice.
    pub fn decode_from_slice(buf: &[u8]) -> Result<Self, prost::DecodeError> {
        ProstMessage::decode(buf)
    }
}

impl MessageEnvelope {
    /// Encodes this message envelope to a protobuf byte vector.
    #[must_use]
    pub fn encode_to_vec(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(self.encoded_len());
        self.encode(&mut buf)
            .expect("encoding into Vec never fails");
        buf
    }

    /// Decodes a `MessageEnvelope` from a byte slice.
    pub fn decode_from_slice(buf: &[u8]) -> Result<Self, prost::DecodeError> {
        ProstMessage::decode(buf)
    }
}

impl SyncRequest {
    /// Encodes this sync request to a protobuf byte vector.
    #[must_use]
    pub fn encode_to_vec(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(self.encoded_len());
        self.encode(&mut buf)
            .expect("encoding into Vec never fails");
        buf
    }

    /// Decodes a `SyncRequest` from a byte slice.
    pub fn decode_from_slice(buf: &[u8]) -> Result<Self, prost::DecodeError> {
        ProstMessage::decode(buf)
    }
}

impl SyncResponse {
    /// Encodes this sync response to a protobuf byte vector.
    #[must_use]
    pub fn encode_to_vec(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(self.encoded_len());
        self.encode(&mut buf)
            .expect("encoding into Vec never fails");
        buf
    }

    /// Decodes a `SyncResponse` from a byte slice.
    pub fn decode_from_slice(buf: &[u8]) -> Result<Self, prost::DecodeError> {
        ProstMessage::decode(buf)
    }
}

impl EncryptedData {
    /// Encodes this encrypted data container to a protobuf byte vector.
    #[must_use]
    pub fn encode_to_vec(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(self.encoded_len());
        self.encode(&mut buf)
            .expect("encoding into Vec never fails");
        buf
    }

    /// Decodes an `EncryptedData` from a byte slice.
    pub fn decode_from_slice(buf: &[u8]) -> Result<Self, prost::DecodeError> {
        ProstMessage::decode(buf)
    }
}
