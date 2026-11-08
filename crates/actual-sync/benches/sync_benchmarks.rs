use criterion::{black_box, criterion_group, criterion_main, Criterion};

use actual_sync::proto::Message;
use actual_sync::timestamp::{HlcClock, Timestamp, DEFAULT_MAX_DRIFT};
use actual_sync::{diff_merkle, TrieNode};

fn bench_hlc_monotonic_clock(c: &mut Criterion) {
    let mut clock = HlcClock::new(Timestamp::new(0, 0, "1"), DEFAULT_MAX_DRIFT);
    let mut current_time = 1000u64;
    c.bench_function("hlc_send_monotonic_tick", |b| {
        b.iter(|| {
            current_time += 1;
            let ts = clock.send(current_time);
            let _ = black_box(ts);
        })
    });
}

fn bench_merkle_trie_insert_and_hash(c: &mut Criterion) {
    let ts1 = Timestamp::parse("2026-11-08T12:00:00.000Z-0000-0123456789ABCDEF").unwrap();
    let ts2 = Timestamp::parse("2026-11-08T12:05:00.000Z-0000-0123456789ABCDEF").unwrap();
    let ts3 = Timestamp::parse("2026-11-08T12:10:00.000Z-0000-0123456789ABCDEF").unwrap();

    c.bench_function("merkle_trie_insert_3_timestamps", |b| {
        b.iter(|| {
            let mut trie = TrieNode::empty();
            trie.insert(&ts1);
            trie.insert(&ts2);
            trie.insert(&ts3);
            black_box(trie.hash);
        })
    });
}

fn bench_merkle_trie_diff(c: &mut Criterion) {
    let mut trie1 = TrieNode::empty();
    let mut trie2 = TrieNode::empty();

    for minute in 0..100 {
        let ts_str = format!(
            "2026-11-08T12:{:02}:00.000Z-0000-0123456789ABCDEF",
            minute % 60
        );
        if let Ok(ts) = Timestamp::parse(&ts_str) {
            trie1.insert(&ts);
            if minute < 90 {
                trie2.insert(&ts);
            }
        }
    }

    c.bench_function("merkle_trie_diff_100_entries", |b| {
        b.iter(|| {
            let diff = diff_merkle(&trie1, &trie2);
            black_box(diff);
        })
    });
}

fn bench_protobuf_message_encoding(c: &mut Criterion) {
    let msg = Message {
        dataset: "transactions".to_string(),
        row: "tx-123456".to_string(),
        column: "amount".to_string(),
        value: "4200".to_string(),
    };

    c.bench_function("protobuf_message_encode_decode", |b| {
        b.iter(|| {
            let encoded = msg.encode_to_vec();
            let decoded = Message::decode_from_slice(&encoded).unwrap();
            black_box(decoded);
        })
    });
}

criterion_group!(
    benches,
    bench_hlc_monotonic_clock,
    bench_merkle_trie_insert_and_hash,
    bench_merkle_trie_diff,
    bench_protobuf_message_encoding
);
criterion_main!(benches);
