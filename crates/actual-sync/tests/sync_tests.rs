use actual_sync::proto::{EncryptedData, Message, MessageEnvelope, SyncRequest, SyncResponse};
use actual_sync::timestamp::{HlcClock, Timestamp, DEFAULT_MAX_DRIFT};
use actual_sync::{diff_merkle, TimestampError, TrieNode};
use chrono::{DateTime, Utc};

#[test]
fn test_timestamp_parsing_and_formatting() {
    let valid_strings = [
        "1970-01-01T00:00:00.000Z-0000-0000000000000000",
        "2015-04-24T22:23:42.123Z-1000-0123456789ABCDEF",
        "9999-12-31T23:59:59.999Z-FFFF-FFFFFFFFFFFFFFFF",
    ];

    for s in valid_strings {
        let ts = Timestamp::parse(s).expect("should parse valid timestamp");
        assert_eq!(ts.to_string(), s);
    }
}

#[test]
fn test_timestamp_invalid_inputs() {
    let invalid_strings = [
        "",
        " ",
        "0",
        "invalid",
        "1969-1-1T0:0:0.0Z-0-0-0",
        "1969-01-01T00:00:00.000Z-0000-0000000000000000",
        "10000-01-01T00:00:00.000Z-FFFF-FFFFFFFFFFFFFFFF",
        "9999-12-31T23:59:59.999Z-10000-FFFFFFFFFFFFFFFF",
        "9999-12-31T23:59:59.999Z-FFFF-10000000000000000",
    ];

    for s in invalid_strings {
        assert!(
            Timestamp::parse(s).is_err(),
            "expected failure for input: {s}"
        );
    }
}

#[test]
fn test_timestamp_comparison_order() {
    let zero = Timestamp::zero();
    let max = Timestamp::max();
    let mid = Timestamp::parse("2026-11-02T14:00:00.000Z-0001-0000000000000001").unwrap();

    assert_eq!(zero, Timestamp::zero());
    assert!(max > zero);
    assert!(mid > zero);
    assert!(mid < max);
}

#[test]
fn test_hlc_send_monotonic() {
    let mut clock = HlcClock::new(Timestamp::new(0, 0, "1"), DEFAULT_MAX_DRIFT);

    // Monotonic advancing physical time
    let ts1 = clock.send(10).unwrap();
    assert_eq!(
        ts1,
        Timestamp::parse("1970-01-01T00:00:00.010Z-0000-0000000000000001").unwrap()
    );

    let ts2 = clock.send(11).unwrap();
    assert_eq!(
        ts2,
        Timestamp::parse("1970-01-01T00:00:00.011Z-0000-0000000000000001").unwrap()
    );

    let ts3 = clock.send(12).unwrap();
    assert_eq!(
        ts3,
        Timestamp::parse("1970-01-01T00:00:00.012Z-0000-0000000000000001").unwrap()
    );
}

#[test]
fn test_hlc_send_stuttering_clock() {
    let mut clock = HlcClock::new(Timestamp::new(0, 0, "1"), DEFAULT_MAX_DRIFT);

    // Stuttering physical time (time does not advance)
    let ts1 = clock.send(20).unwrap();
    assert_eq!(
        ts1,
        Timestamp::parse("1970-01-01T00:00:00.020Z-0000-0000000000000001").unwrap()
    );

    let ts2 = clock.send(20).unwrap();
    assert_eq!(
        ts2,
        Timestamp::parse("1970-01-01T00:00:00.020Z-0001-0000000000000001").unwrap()
    );

    let ts3 = clock.send(20).unwrap();
    assert_eq!(
        ts3,
        Timestamp::parse("1970-01-01T00:00:00.020Z-0002-0000000000000001").unwrap()
    );

    let ts4 = clock.send(21).unwrap();
    assert_eq!(
        ts4,
        Timestamp::parse("1970-01-01T00:00:00.021Z-0000-0000000000000001").unwrap()
    );
}

#[test]
fn test_hlc_send_regressing_clock() {
    let mut clock = HlcClock::new(Timestamp::new(0, 0, "1"), DEFAULT_MAX_DRIFT);

    let ts1 = clock.send(30).unwrap();
    assert_eq!(
        ts1,
        Timestamp::parse("1970-01-01T00:00:00.030Z-0000-0000000000000001").unwrap()
    );

    // Regressing time (wall clock moved backward to 29)
    let ts2 = clock.send(29).unwrap();
    assert_eq!(
        ts2,
        Timestamp::parse("1970-01-01T00:00:00.030Z-0001-0000000000000001").unwrap()
    );

    let ts3 = clock.send(29).unwrap();
    assert_eq!(
        ts3,
        Timestamp::parse("1970-01-01T00:00:00.030Z-0002-0000000000000001").unwrap()
    );

    let ts4 = clock.send(31).unwrap();
    assert_eq!(
        ts4,
        Timestamp::parse("1970-01-01T00:00:00.031Z-0000-0000000000000001").unwrap()
    );
}

#[test]
fn test_hlc_clock_drift_exceeded() {
    let mut clock = HlcClock::new(Timestamp::new(0, 0, "1"), 5 * 60 * 1000);

    // Advance clock to 10 minutes ahead
    clock.set_timestamp(Timestamp::new(10 * 60 * 1000, 0, "1"));

    // Physical time is at 0 (10 minutes behind > 5 min max drift)
    let res = clock.send(0);
    assert!(matches!(res, Err(TimestampError::ClockDrift { .. })));
}

#[test]
fn test_hlc_counter_overflow() {
    let mut clock = HlcClock::new(Timestamp::new(100, 0xFFFF, "1"), DEFAULT_MAX_DRIFT);

    // Attempting to increment when counter is 0xFFFF at same time
    let res = clock.send(100);
    assert_eq!(res, Err(TimestampError::Overflow));
}

#[test]
fn test_hlc_receive_merges_monotonically() {
    let mut clock = HlcClock::new(Timestamp::new(0, 0, "1"), DEFAULT_MAX_DRIFT);

    let msg1 = Timestamp::parse("1970-01-01T00:00:00.051Z-0000-0000000000000002").unwrap();
    let recv1 = clock.recv(&msg1, 52).unwrap();
    assert_eq!(
        recv1,
        Timestamp::parse("1970-01-01T00:00:00.052Z-0000-0000000000000001").unwrap()
    );

    let msg2 = Timestamp::parse("1970-01-01T00:00:00.053Z-0000-0000000000000002").unwrap();
    let recv2 = clock.recv(&msg2, 54).unwrap();
    assert_eq!(
        recv2,
        Timestamp::parse("1970-01-01T00:00:00.054Z-0000-0000000000000001").unwrap()
    );
}

#[test]
fn test_merkle_insert_and_hash_upstream_vector() {
    let mut trie = TrieNode::empty();

    let ts1 = Timestamp::parse("2018-11-12T13:21:40.122Z-0000-0123456789ABCDEF").unwrap();
    let ts2 = Timestamp::parse("2018-11-13T13:21:40.122Z-0000-0123456789ABCDEF").unwrap();

    assert_eq!(ts1.hash(), 1_983_295_247);
    assert_eq!(ts2.hash(), 1_469_038_940);

    trie.insert(&ts1);
    trie.insert(&ts2);

    // 1983295247 ^ 1469038940 = 565800531
    assert_eq!(trie.hash, 565_800_531);
}

#[test]
fn test_merkle_diff_known_time_difference() {
    // Helper to insert a timestamp with a mock custom hash into a trie
    fn insert_mock(trie: &mut TrieNode, ts_str: &str, mock_hash: u32) {
        let ts = Timestamp::parse(ts_str).unwrap();
        let minutes = ts.millis() / 1000 / 60;
        let key = actual_sync::merkle::to_base3(minutes);

        trie.hash ^= mock_hash;

        let mut curr = trie;
        for c in key.chars() {
            let idx = match c {
                '0' => 0,
                '1' => 1,
                '2' => 2,
                _ => continue,
            };
            let child = curr.children[idx].get_or_insert_with(|| Box::new(TrieNode::empty()));
            child.hash ^= mock_hash;
            curr = curr.children[idx].as_deref_mut().unwrap();
        }
    }

    let mut trie1 = TrieNode::empty();
    let mut trie2 = TrieNode::empty();

    // Client 1 messages
    insert_mock(
        &mut trie1,
        "2018-11-13T13:20:40.122Z-0000-0123456789ABCDEF",
        1000,
    );
    insert_mock(
        &mut trie1,
        "2018-11-14T13:05:35.122Z-0000-0123456789ABCDEF",
        1100,
    );
    insert_mock(
        &mut trie1,
        "2018-11-15T22:19:00.122Z-0000-0123456789ABCDEF",
        1200,
    );
    assert_eq!(trie1.hash, 788);

    // Client 2 messages
    insert_mock(
        &mut trie2,
        "2018-11-20T13:19:40.122Z-0000-0123456789ABCDEF",
        1300,
    );
    insert_mock(
        &mut trie2,
        "2018-11-25T13:19:40.122Z-0000-0123456789ABCDEF",
        1400,
    );
    assert_eq!(trie2.hash, 108);

    // Diff returns 2018-11-02T17:15:00.000Z
    let diff_time = diff_merkle(&trie1, &trie2).expect("diff should find divergence");
    let dt = DateTime::<Utc>::from_timestamp_millis(diff_time as i64).unwrap();
    assert_eq!(
        dt.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string(),
        "2018-11-02T17:15:00.000Z"
    );

    // Once both sync up by inserting missing messages, tries become identical
    insert_mock(
        &mut trie1,
        "2018-11-20T13:19:40.122Z-0000-0123456789ABCDEF",
        1300,
    );
    insert_mock(
        &mut trie1,
        "2018-11-25T13:19:40.122Z-0000-0123456789ABCDEF",
        1400,
    );

    insert_mock(
        &mut trie2,
        "2018-11-13T13:20:40.122Z-0000-0123456789ABCDEF",
        1000,
    );
    insert_mock(
        &mut trie2,
        "2018-11-14T13:05:35.122Z-0000-0123456789ABCDEF",
        1100,
    );
    insert_mock(
        &mut trie2,
        "2018-11-15T22:19:00.122Z-0000-0123456789ABCDEF",
        1200,
    );

    assert_eq!(trie1.hash, 888);
    assert_eq!(trie1.hash, trie2.hash);
    assert_eq!(diff_merkle(&trie1, &trie2), None);
}

#[test]
fn test_merkle_empty_diff_returns_zero() {
    let trie1 = TrieNode::empty();
    let mut trie2 = TrieNode::empty();
    trie2.insert(&Timestamp::parse("2009-01-02T10:17:37.789Z-0000-0000testinguuid1").unwrap());

    assert_eq!(diff_merkle(&trie1, &trie2), Some(0));
    assert_eq!(diff_merkle(&trie2, &trie1), Some(0));
}

#[test]
fn test_merkle_pruning_preserves_root_hash() {
    let mut trie = TrieNode::empty();
    let timestamps = [
        "2018-11-01T01:00:00.000Z-0000-0123456789ABCDEF",
        "2018-11-01T01:09:00.000Z-0000-0123456789ABCDEF",
        "2018-11-01T01:18:00.000Z-0000-0123456789ABCDEF",
        "2018-11-01T01:27:00.000Z-0000-0123456789ABCDEF",
    ];

    for s in timestamps {
        trie.insert(&Timestamp::parse(s).unwrap());
    }

    let initial_hash = trie.hash;
    let pruned = trie.prune(2);
    assert_eq!(pruned.hash, initial_hash);
}

#[test]
fn test_merkle_json_serialization_roundtrip() {
    let mut trie = TrieNode::empty();
    trie.insert(&Timestamp::parse("2018-11-12T13:21:40.122Z-0000-0123456789ABCDEF").unwrap());
    trie.insert(&Timestamp::parse("2018-11-13T13:21:40.122Z-0000-0123456789ABCDEF").unwrap());

    let json_str = serde_json::to_string(&trie).unwrap();
    assert!(json_str.contains("\"hash\":565800531"));

    let deserialized: TrieNode = serde_json::from_str(&json_str).unwrap();
    assert_eq!(trie, deserialized);
}

#[test]
fn test_protobuf_codecs_roundtrip() {
    let msg = Message {
        dataset: "transactions".to_string(),
        row: "tx-123".to_string(),
        column: "amount".to_string(),
        value: "4200".to_string(),
    };

    let encoded_msg = msg.encode_to_vec();
    let decoded_msg = Message::decode_from_slice(&encoded_msg).unwrap();
    assert_eq!(msg, decoded_msg);

    let envelope = MessageEnvelope {
        timestamp: "2026-11-02T14:00:00.000Z-0000-0000000000000001".to_string(),
        is_encrypted: false,
        content: encoded_msg,
    };

    let encoded_env = envelope.encode_to_vec();
    let decoded_env = MessageEnvelope::decode_from_slice(&encoded_env).unwrap();
    assert_eq!(envelope, decoded_env);

    let sync_req = SyncRequest {
        messages: vec![envelope.clone()],
        file_id: "file-xyz".to_string(),
        group_id: "group-abc".to_string(),
        key_id: "key-1".to_string(),
        since: "2026-11-01T00:00:00.000Z-0000-0000000000000000".to_string(),
    };

    let encoded_req = sync_req.encode_to_vec();
    let decoded_req = SyncRequest::decode_from_slice(&encoded_req).unwrap();
    assert_eq!(sync_req, decoded_req);

    let sync_res = SyncResponse {
        messages: vec![envelope],
        merkle: "{\"hash\":565800531}".to_string(),
    };

    let encoded_res = sync_res.encode_to_vec();
    let decoded_res = SyncResponse::decode_from_slice(&encoded_res).unwrap();
    assert_eq!(sync_res, decoded_res);

    let encrypted = EncryptedData {
        iv: vec![1, 2, 3, 4],
        auth_tag: vec![5, 6, 7, 8],
        data: vec![9, 10, 11, 12],
    };

    let encoded_enc = encrypted.encode_to_vec();
    let decoded_enc = EncryptedData::decode_from_slice(&encoded_enc).unwrap();
    assert_eq!(encrypted, decoded_enc);
}
