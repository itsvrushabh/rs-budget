//! Hybrid Logical Clock (HLC) Timestamp implementation matching upstream Actual Budget.

use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;

use crate::error::TimestampError;
use crate::hash::murmur3_32;

/// Maximum counter value allowed (16-bit unsigned integer, `0xFFFF`).
pub const MAX_COUNTER: u16 = 0xFFFF;

/// Default maximum allowed clock drift: 5 minutes in milliseconds.
pub const DEFAULT_MAX_DRIFT: u64 = 5 * 60 * 1000;

/// Maximum allowed milliseconds (corresponds to year 9999).
pub const MAX_MILLIS: u64 = 253_402_300_799_999;

/// A globally unique, monotonic timestamp generated from physical time,
/// a logical counter, and a client node identifier.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp {
    millis: u64,
    counter: u16,
    node: String,
}

impl Timestamp {
    /// Creates a new `Timestamp`.
    ///
    /// The `node` parameter is formatted to 16 characters padded with leading zeros.
    #[must_use]
    pub fn new(millis: u64, counter: u16, node: &str) -> Self {
        let node_formatted = if node.len() < 16 {
            format!("{:0>16}", node)
        } else {
            node[node.len() - 16..].to_string()
        };

        Self {
            millis,
            counter,
            node: node_formatted,
        }
    }

    /// Returns the physical/logical milliseconds since Unix epoch.
    #[must_use]
    pub const fn millis(&self) -> u64 {
        self.millis
    }

    /// Returns the logical counter.
    #[must_use]
    pub const fn counter(&self) -> u16 {
        self.counter
    }

    /// Returns the 16-character node identifier.
    #[must_use]
    pub fn node(&self) -> &str {
        &self.node
    }

    /// Computes the MurmurHash3 (32-bit) hash of this timestamp's string representation.
    #[must_use]
    pub fn hash(&self) -> u32 {
        murmur3_32(self.to_string().as_bytes(), 0)
    }

    /// Zero / minimum timestamp (`1970-01-01T00:00:00.000Z-0000-0000000000000000`).
    #[must_use]
    pub fn zero() -> Self {
        Self {
            millis: 0,
            counter: 0,
            node: "0000000000000000".to_string(),
        }
    }

    /// Maximum timestamp (`9999-12-31T23:59:59.999Z-FFFF-FFFFFFFFFFFFFFFF`).
    #[must_use]
    pub fn max() -> Self {
        Self {
            millis: MAX_MILLIS,
            counter: MAX_COUNTER,
            node: "FFFFFFFFFFFFFFFF".to_string(),
        }
    }

    /// Formats an ISO-8601 string into a starting timestamp search key (`since`).
    #[must_use]
    pub fn since(iso_string: &str) -> String {
        format!("{iso_string}-0000-0000000000000000")
    }

    /// Parses a 46-character string into a `Timestamp`.
    ///
    /// # Errors
    /// Returns [`TimestampError::Invalid`] if the string is malformed, has invalid dates,
    /// or counter/node values out of range.
    pub fn parse(s: &str) -> Result<Self, TimestampError> {
        let parts: Vec<&str> = s.split('-').collect();
        if parts.len() != 5 {
            return Err(TimestampError::Invalid(s.to_string()));
        }

        let iso_date_str = format!("{}-{}-{}", parts[0], parts[1], parts[2]);
        let dt = match DateTime::parse_from_rfc3339(&iso_date_str) {
            Ok(d) => d.with_timezone(&Utc),
            Err(_) => return Err(TimestampError::Invalid(s.to_string())),
        };

        let millis = dt.timestamp_millis();
        if millis < 0 || millis as u64 > MAX_MILLIS {
            return Err(TimestampError::Invalid(s.to_string()));
        }

        let counter = match u16::from_str_radix(parts[3], 16) {
            Ok(c) => c,
            Err(_) => return Err(TimestampError::Invalid(s.to_string())),
        };

        if parts[3].len() != 4 {
            return Err(TimestampError::Invalid(s.to_string()));
        }

        let node = parts[4];
        if node.is_empty() || node.len() > 16 {
            return Err(TimestampError::Invalid(s.to_string()));
        }

        Ok(Self::new(millis as u64, counter, node))
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let dt = Utc
            .timestamp_millis_opt(self.millis as i64)
            .single()
            .unwrap_or_else(Utc::now);

        let iso = dt.format("%Y-%m-%dT%H:%M:%S%.3fZ");
        write!(f, "{}-{:04X}-{:0>16}", iso, self.counter, self.node)
    }
}

impl FromStr for Timestamp {
    type Err = TimestampError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl Serialize for Timestamp {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Timestamp {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::parse(&s).map_err(serde::de::Error::custom)
    }
}

/// Generates a random 16-character hexadecimal client node identifier.
#[must_use]
pub fn make_client_id() -> String {
    let raw = uuid::Uuid::new_v4().simple().to_string();
    raw[raw.len() - 16..].to_string()
}

/// A Hybrid Logical Clock (HLC) instance tracking local monotonic time.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HlcClock {
    timestamp: Timestamp,
    max_drift: u64,
}

impl HlcClock {
    /// Creates a new `HlcClock` initialized with the given timestamp and max drift.
    #[must_use]
    pub fn new(timestamp: Timestamp, max_drift: u64) -> Self {
        Self {
            timestamp,
            max_drift,
        }
    }

    /// Creates a clock with default 5-minute drift and a freshly generated client ID.
    #[must_use]
    pub fn with_client_id(client_id: &str) -> Self {
        Self::new(Timestamp::new(0, 0, client_id), DEFAULT_MAX_DRIFT)
    }

    /// Returns the current clock timestamp.
    #[must_use]
    pub const fn timestamp(&self) -> &Timestamp {
        &self.timestamp
    }

    /// Sets the current timestamp directly.
    pub fn set_timestamp(&mut self, timestamp: Timestamp) {
        self.timestamp = timestamp;
    }

    /// Returns the configured maximum drift in milliseconds.
    #[must_use]
    pub const fn max_drift(&self) -> u64 {
        self.max_drift
    }

    /// Advances the clock to generate a new unique, monotonic timestamp for sending a message.
    ///
    /// # Errors
    /// Returns [`TimestampError::ClockDrift`] if the clock drift exceeds `max_drift`,
    /// or [`TimestampError::Overflow`] if the counter exceeds `0xFFFF`.
    pub fn send(&mut self, now_millis: u64) -> Result<Timestamp, TimestampError> {
        let l_old = self.timestamp.millis();
        let c_old = self.timestamp.counter();

        let l_new = l_old.max(now_millis);
        let c_new = if l_old == l_new {
            c_old.checked_add(1)
        } else {
            Some(0)
        };

        if l_new.saturating_sub(now_millis) > self.max_drift {
            return Err(TimestampError::ClockDrift {
                logical: l_new,
                physical: now_millis,
                max_drift: self.max_drift,
            });
        }

        let c_new = c_new.ok_or(TimestampError::Overflow)?;

        self.timestamp = Timestamp::new(l_new, c_new, self.timestamp.node());
        Ok(self.timestamp.clone())
    }

    /// Receives and merges an incoming message timestamp with local physical and logical time.
    ///
    /// # Errors
    /// Returns [`TimestampError::ClockDrift`] if clock drift exceeds `max_drift`,
    /// or [`TimestampError::Overflow`] if the counter exceeds `0xFFFF`.
    pub fn recv(&mut self, msg: &Timestamp, now_millis: u64) -> Result<Timestamp, TimestampError> {
        let l_msg = msg.millis();
        let c_msg = msg.counter();

        if l_msg.saturating_sub(now_millis) > self.max_drift {
            return Err(TimestampError::ClockDrift {
                logical: l_msg,
                physical: now_millis,
                max_drift: self.max_drift,
            });
        }

        let l_old = self.timestamp.millis();
        let c_old = self.timestamp.counter();

        let l_new = l_old.max(now_millis).max(l_msg);
        let c_new = if l_new == l_old && l_new == l_msg {
            c_old.max(c_msg).checked_add(1)
        } else if l_new == l_old {
            c_old.checked_add(1)
        } else if l_new == l_msg {
            c_msg.checked_add(1)
        } else {
            Some(0)
        };

        if l_new.saturating_sub(now_millis) > self.max_drift {
            return Err(TimestampError::ClockDrift {
                logical: l_new,
                physical: now_millis,
                max_drift: self.max_drift,
            });
        }

        let c_new = c_new.ok_or(TimestampError::Overflow)?;

        self.timestamp = Timestamp::new(l_new, c_new, self.timestamp.node());
        Ok(self.timestamp.clone())
    }
}
