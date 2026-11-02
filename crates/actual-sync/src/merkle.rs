//! Trinary Radix Merkle Trie for CRDT synchronization diffing.

use serde::de::{MapAccess, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

use crate::timestamp::Timestamp;

/// Represents a node within a trinary radix trie (keys '0', '1', '2').
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrieNode {
    pub hash: u32,
    pub children: [Option<Box<TrieNode>>; 3],
}

impl Default for TrieNode {
    fn default() -> Self {
        Self::empty()
    }
}

impl TrieNode {
    /// Creates an empty TrieNode with hash 0 and no children.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            hash: 0,
            children: [None, None, None],
        }
    }

    /// Returns a reference to a child node if it exists ('0', '1', or '2').
    #[must_use]
    pub fn child(&self, key: char) -> Option<&TrieNode> {
        let idx = match key {
            '0' => 0,
            '1' => 1,
            '2' => 2,
            _ => return None,
        };
        self.children[idx].as_deref()
    }

    /// Returns a mutable reference to a child node if it exists.
    pub fn child_mut(&mut self, key: char) -> Option<&mut TrieNode> {
        let idx = match key {
            '0' => 0,
            '1' => 1,
            '2' => 2,
            _ => return None,
        };
        self.children[idx].as_deref_mut()
    }

    /// Sets or replaces a child node for key ('0', '1', or '2').
    pub fn set_child(&mut self, key: char, child: Option<TrieNode>) {
        let idx = match key {
            '0' => 0,
            '1' => 1,
            '2' => 2,
            _ => return,
        };
        self.children[idx] = child.map(Box::new);
    }

    /// Returns the present child keys in ascending order (`'0'`, `'1'`, `'2'`).
    #[must_use]
    pub fn keys(&self) -> Vec<char> {
        let mut keys = Vec::with_capacity(3);
        if self.children[0].is_some() {
            keys.push('0');
        }
        if self.children[1].is_some() {
            keys.push('1');
        }
        if self.children[2].is_some() {
            keys.push('2');
        }
        keys
    }

    /// Inserts a timestamp into this trie, updating hashes along the path.
    pub fn insert(&mut self, timestamp: &Timestamp) {
        let hash = timestamp.hash();
        let minutes = timestamp.millis() / 1000 / 60;
        let key = to_base3(minutes);

        self.hash ^= hash;
        insert_key(self, &key, hash);
    }

    /// Inserts a timestamp and returns a new cloned trie.
    #[must_use]
    pub fn inserted(&self, timestamp: &Timestamp) -> Self {
        let mut copy = self.clone();
        copy.insert(timestamp);
        copy
    }

    /// Prunes the trie, keeping only the last `n` child branches while preserving hashes.
    #[must_use]
    pub fn prune(&self, n: usize) -> Self {
        if self.hash == 0 {
            return self.clone();
        }

        let mut next = Self {
            hash: self.hash,
            children: [None, None, None],
        };

        let keys = self.keys();
        let start = keys.len().saturating_sub(n);

        for &k in &keys[start..] {
            if let Some(child) = self.child(k) {
                next.set_child(k, Some(child.prune(n)));
            }
        }

        next
    }
}

/// Recursively inserts a key into a TrieNode.
fn insert_key(node: &mut TrieNode, key: &str, hash: u32) {
    if key.is_empty() {
        return;
    }

    let c = key.chars().next().unwrap();
    let idx = match c {
        '0' => 0,
        '1' => 1,
        '2' => 2,
        _ => return,
    };

    let child = node.children[idx].get_or_insert_with(|| Box::new(TrieNode::empty()));
    child.hash ^= hash;
    insert_key(child, &key[1..], hash);
}

/// Builds a Merkle trie from an iterator of timestamps.
pub fn build<'a>(timestamps: impl IntoIterator<Item = &'a Timestamp>) -> TrieNode {
    let mut trie = TrieNode::empty();
    for ts in timestamps {
        trie.insert(ts);
    }
    trie
}

/// Computes the earliest divergent time (in milliseconds since epoch) between two Merkle tries.
///
/// Returns `None` if the tries are identical in state (hashes match).
#[must_use]
pub fn diff(trie1: &TrieNode, trie2: &TrieNode) -> Option<u64> {
    if trie1.hash == trie2.hash {
        return None;
    }

    let mut node1 = trie1;
    let mut node2 = trie2;
    let mut k = String::new();

    let empty = TrieNode::empty();

    loop {
        let mut keyset = std::collections::BTreeSet::new();
        keyset.extend(node1.keys());
        keyset.extend(node2.keys());

        let mut diffkey: Option<char> = None;

        for key in keyset {
            let next1 = node1.child(key);
            let next2 = node2.child(key);

            if next1.is_none() || next2.is_none() {
                break;
            }

            if next1.unwrap().hash != next2.unwrap().hash {
                diffkey = Some(key);
                break;
            }
        }

        let Some(dk) = diffkey else {
            return Some(key_to_timestamp(&k));
        };

        k.push(dk);
        node1 = node1.child(dk).unwrap_or(&empty);
        node2 = node2.child(dk).unwrap_or(&empty);
    }
}

/// Converts a base-3 trie key into physical time in milliseconds since epoch.
#[must_use]
pub fn key_to_timestamp(key: &str) -> u64 {
    let mut fullkey = key.to_string();
    if fullkey.len() < 16 {
        fullkey.push_str(&"0".repeat(16 - fullkey.len()));
    }

    let minutes = from_base3(&fullkey);
    minutes * 1000 * 60
}

/// Converts a 64-bit integer into a base-3 string.
#[must_use]
pub fn to_base3(mut n: u64) -> String {
    if n == 0 {
        return "0".to_string();
    }
    let mut digits = Vec::new();
    while n > 0 {
        digits.push(b'0' + (n % 3) as u8);
        n /= 3;
    }
    digits.reverse();
    // Safety: digits contains only '0', '1', '2'
    String::from_utf8(digits).unwrap()
}

/// Parses a base-3 string into a 64-bit integer.
#[must_use]
pub fn from_base3(s: &str) -> u64 {
    let mut val: u64 = 0;
    for c in s.chars() {
        let digit = match c {
            '0' => 0,
            '1' => 1,
            '2' => 2,
            _ => continue,
        };
        val = val.saturating_mul(3).saturating_add(digit);
    }
    val
}

impl Serialize for TrieNode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let child_count = self.children.iter().filter(|c| c.is_some()).count();
        let mut map = serializer.serialize_map(Some(child_count + 1))?;

        if let Some(child0) = &self.children[0] {
            map.serialize_entry("0", child0)?;
        }
        if let Some(child1) = &self.children[1] {
            map.serialize_entry("1", child1)?;
        }
        if let Some(child2) = &self.children[2] {
            map.serialize_entry("2", child2)?;
        }

        map.serialize_entry("hash", &self.hash)?;
        map.end()
    }
}

impl<'de> Deserialize<'de> for TrieNode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct TrieNodeVisitor;

        impl<'de> Visitor<'de> for TrieNodeVisitor {
            type Value = TrieNode;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a JSON map representing a TrieNode")
            }

            fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut hash = 0u32;
                let mut child0 = None;
                let mut child1 = None;
                let mut child2 = None;

                while let Some(key) = access.next_key::<String>()? {
                    match key.as_str() {
                        "hash" => {
                            hash = access.next_value()?;
                        }
                        "0" => {
                            child0 = Some(Box::new(access.next_value()?));
                        }
                        "1" => {
                            child1 = Some(Box::new(access.next_value()?));
                        }
                        "2" => {
                            child2 = Some(Box::new(access.next_value()?));
                        }
                        _ => {
                            let _ = access.next_value::<serde_json::Value>()?;
                        }
                    }
                }

                Ok(TrieNode {
                    hash,
                    children: [child0, child1, child2],
                })
            }
        }

        deserializer.deserialize_map(TrieNodeVisitor)
    }
}
