// This file is generated from `specs/proto/sync.proto` via prost-build.

#[derive(Clone, PartialEq, ::prost::Message, serde::Serialize, serde::Deserialize)]
pub struct EncryptedData {
    #[prost(bytes = "vec", tag = "1")]
    pub iv: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    pub auth_tag: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "3")]
    pub data: ::prost::alloc::vec::Vec<u8>,
}

#[derive(Clone, PartialEq, ::prost::Message, serde::Serialize, serde::Deserialize)]
pub struct Message {
    #[prost(string, tag = "1")]
    pub dataset: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub row: ::prost::alloc::string::String,
    #[prost(string, tag = "3")]
    pub column: ::prost::alloc::string::String,
    #[prost(string, tag = "4")]
    pub value: ::prost::alloc::string::String,
}

#[derive(Clone, PartialEq, ::prost::Message, serde::Serialize, serde::Deserialize)]
pub struct MessageEnvelope {
    #[prost(string, tag = "1")]
    pub timestamp: ::prost::alloc::string::String,
    #[prost(bool, tag = "2")]
    pub is_encrypted: bool,
    #[prost(bytes = "vec", tag = "3")]
    pub content: ::prost::alloc::vec::Vec<u8>,
}

#[derive(Clone, PartialEq, ::prost::Message, serde::Serialize, serde::Deserialize)]
pub struct SyncRequest {
    #[prost(message, repeated, tag = "1")]
    pub messages: ::prost::alloc::vec::Vec<MessageEnvelope>,
    #[prost(string, tag = "2")]
    pub file_id: ::prost::alloc::string::String,
    #[prost(string, tag = "3")]
    pub group_id: ::prost::alloc::string::String,
    #[prost(string, tag = "5")]
    pub key_id: ::prost::alloc::string::String,
    #[prost(string, tag = "6")]
    pub since: ::prost::alloc::string::String,
}

#[derive(Clone, PartialEq, ::prost::Message, serde::Serialize, serde::Deserialize)]
pub struct SyncResponse {
    #[prost(message, repeated, tag = "1")]
    pub messages: ::prost::alloc::vec::Vec<MessageEnvelope>,
    #[prost(string, tag = "2")]
    pub merkle: ::prost::alloc::string::String,
}
