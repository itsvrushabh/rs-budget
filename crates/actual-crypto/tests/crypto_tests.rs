use actual_crypto::{
    decrypt, encrypt, encrypt_with_iv, CryptoError, EncryptedData, Key, KeyStore, IV_LENGTH_BYTES,
};
use base64::Engine;
use std::fs;

#[test]
fn test_pbkdf2_against_known_answer_vector() {
    let fixture_content =
        fs::read_to_string("../../fixtures/crypto/test_vectors.json").expect("Read test vectors");
    let json: serde_json::Value =
        serde_json::from_str(&fixture_content).expect("Parse test vectors JSON");

    let pbkdf2_vec = &json["pbkdf2"][0];
    let password = pbkdf2_vec["password"].as_str().unwrap();
    let salt = pbkdf2_vec["salt"].as_str().unwrap();
    let expected_key_hex = pbkdf2_vec["expected_key_hex"].as_str().unwrap();
    let expected_key_base64 = pbkdf2_vec["expected_key_base64"].as_str().unwrap();

    let key = Key::from_password(Some("test-key-id".to_string()), password, salt)
        .expect("Derive key from password");

    assert_eq!(hex::encode(key.raw()), expected_key_hex);
    assert_eq!(key.base64(), expected_key_base64);
    assert_eq!(key.id(), "test-key-id");
}

#[test]
fn test_aes_gcm_against_known_answer_vector() {
    let fixture_content =
        fs::read_to_string("../../fixtures/crypto/test_vectors.json").expect("Read test vectors");
    let json: serde_json::Value =
        serde_json::from_str(&fixture_content).expect("Parse test vectors JSON");

    let aes_vec = &json["aes_gcm"][0];
    let key_hex = aes_vec["key_hex"].as_str().unwrap();
    let iv_hex = aes_vec["iv_hex"].as_str().unwrap();
    let plaintext = aes_vec["plaintext"].as_str().unwrap();
    let expected_ciphertext_hex = aes_vec["ciphertext_hex"].as_str().unwrap();
    let expected_auth_tag_hex = aes_vec["auth_tag_hex"].as_str().unwrap();

    let key_raw: [u8; 32] = hex::decode(key_hex)
        .unwrap()
        .try_into()
        .expect("32 bytes key");
    let key = Key::new(Some("test-aes-key".to_string()), key_raw);

    let iv_raw: [u8; IV_LENGTH_BYTES] = hex::decode(iv_hex)
        .unwrap()
        .try_into()
        .expect("12 bytes IV");

    // Encrypt with fixed IV
    let encrypted = encrypt_with_iv(&key, plaintext.as_bytes(), &iv_raw).expect("Encrypt with IV");

    assert_eq!(hex::encode(&encrypted.value), expected_ciphertext_hex);

    let auth_tag_bytes = base64::prelude::BASE64_STANDARD
        .decode(encrypted.meta.auth_tag.trim())
        .unwrap();
    assert_eq!(hex::encode(&auth_tag_bytes), expected_auth_tag_hex);

    // Decrypt and verify
    let decrypted = decrypt(&key, &encrypted).expect("Decrypt test vector");
    assert_eq!(String::from_utf8(decrypted).unwrap(), plaintext);
}

#[test]
fn test_roundtrip_encryption_decryption() {
    let key =
        Key::from_password(None, "my secret password", "user_unique_salt").expect("Derive key");

    let messages = [
        "",
        "Hello, Actual Budget Rust Rewrite!",
        "{\"dataset\":\"transactions\",\"row\":\"t-100\",\"column\":\"notes\",\"value\":\"Dinner with friends\"}",
        "A somewhat longer message that spans across multiple AES blocks to verify block alignment and padding.",
    ];

    for msg in messages {
        let encrypted = encrypt(&key, msg.as_bytes()).expect("Encrypt message");
        let decrypted = decrypt(&key, &encrypted).expect("Decrypt message");
        assert_eq!(String::from_utf8(decrypted).unwrap(), msg);
    }
}

#[test]
fn test_tampered_ciphertext_fails_auth() {
    let key = Key::from_password(None, "password", "salt").unwrap();
    let mut encrypted = encrypt(&key, b"Sensitive financial data").unwrap();

    // Tamper with first byte
    encrypted.value[0] ^= 0x55;

    let result = decrypt(&key, &encrypted);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), CryptoError::Decryption(_)));
}

#[test]
fn test_tampered_auth_tag_fails() {
    let key = Key::from_password(None, "password", "salt").unwrap();
    let mut encrypted = encrypt(&key, b"Sensitive financial data").unwrap();

    // Tamper with auth tag
    let mut tag_bytes = base64::prelude::BASE64_STANDARD
        .decode(&encrypted.meta.auth_tag)
        .unwrap();
    tag_bytes[0] ^= 0xFF;
    encrypted.meta.auth_tag = base64::prelude::BASE64_STANDARD.encode(&tag_bytes);

    let result = decrypt(&key, &encrypted);
    assert!(result.is_err());
}

#[test]
fn test_wrong_key_fails_decryption() {
    let key1 = Key::from_password(None, "password-1", "salt").unwrap();
    let key2 = Key::from_password(None, "password-2", "salt").unwrap();

    let encrypted = encrypt(&key1, b"Secret transaction").unwrap();
    let result = decrypt(&key2, &encrypted);
    assert!(result.is_err());
}

#[test]
fn test_key_store_operations() {
    let mut store = KeyStore::new();

    let key = Key::from_password(Some("master-key-1".to_string()), "pwd", "salt").unwrap();
    assert!(!store.has_key("master-key-1"));

    store.load_key(key.clone());
    assert!(store.has_key("master-key-1"));
    assert_eq!(store.get_key("master-key-1").unwrap().id(), "master-key-1");

    // Encrypt & decrypt via store
    let encrypted = store
        .encrypt("master-key-1", b"Account balance: 100000")
        .unwrap();
    let decrypted = store.decrypt(&encrypted).unwrap();
    assert_eq!(
        String::from_utf8(decrypted).unwrap(),
        "Account balance: 100000"
    );

    // Unload key
    store.unload_key("master-key-1");
    assert!(!store.has_key("master-key-1"));
    assert!(matches!(
        store.get_key("master-key-1").unwrap_err(),
        CryptoError::KeyNotFound(_)
    ));
}

#[test]
fn test_encrypted_data_json_serde() {
    let key = Key::from_password(Some("k-123".to_string()), "pwd", "salt").unwrap();
    let encrypted = encrypt(&key, b"Payload").unwrap();

    let json_str = serde_json::to_string_pretty(&encrypted).expect("Serialize to JSON");

    // Verify camelCase fields in JSON
    assert!(json_str.contains("\"keyId\": \"k-123\""));
    assert!(json_str.contains("\"algorithm\": \"aes-256-gcm\""));
    assert!(json_str.contains("\"iv\":"));
    assert!(json_str.contains("\"authTag\":"));
    assert!(json_str.contains("\"value\":"));

    let deserialized: EncryptedData =
        serde_json::from_str(&json_str).expect("Deserialize from JSON");
    assert_eq!(encrypted, deserialized);
}
