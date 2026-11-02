//! Spreadsheet cell value types.

use serde::{Deserialize, Serialize};
use std::fmt;

use crate::cents::Cents;

/// Cell value stored and evaluated in the spreadsheet DAG.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CellValue {
    /// Monetary cents value.
    Cents(Cents),
    /// Boolean flag (e.g. carryover).
    Bool(bool),
    /// Integer number.
    Int(i64),
    /// Text string.
    Text(String),
}

impl Default for CellValue {
    fn default() -> Self {
        Self::Cents(Cents::ZERO)
    }
}

impl CellValue {
    /// Converts or unboxes the value into `Cents`.
    #[must_use]
    pub fn as_cents(&self) -> Cents {
        match self {
            Self::Cents(c) => *c,
            Self::Int(i) => Cents::from_cents(*i),
            _ => Cents::ZERO,
        }
    }

    /// Converts or unboxes the value into `bool`.
    #[must_use]
    pub fn as_bool(&self) -> bool {
        match self {
            Self::Bool(b) => *b,
            Self::Cents(c) => !c.is_zero(),
            Self::Int(i) => *i != 0,
            Self::Text(s) => !s.is_empty(),
        }
    }

    /// Converts or unboxes the value into `i64`.
    #[must_use]
    pub fn as_int(&self) -> i64 {
        match self {
            Self::Int(i) => *i,
            Self::Cents(c) => c.as_cents(),
            Self::Bool(b) => {
                if *b {
                    1
                } else {
                    0
                }
            }
            Self::Text(_) => 0,
        }
    }

    /// Returns a string slice if the value is text.
    #[must_use]
    pub fn as_text(&self) -> &str {
        match self {
            Self::Text(s) => s,
            _ => "",
        }
    }
}

impl fmt::Display for CellValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cents(c) => write!(f, "{c}"),
            Self::Bool(b) => write!(f, "{b}"),
            Self::Int(i) => write!(f, "{i}"),
            Self::Text(s) => write!(f, "{s}"),
        }
    }
}

impl From<Cents> for CellValue {
    fn from(c: Cents) -> Self {
        Self::Cents(c)
    }
}

impl From<bool> for CellValue {
    fn from(b: bool) -> Self {
        Self::Bool(b)
    }
}

impl From<i64> for CellValue {
    fn from(i: i64) -> Self {
        Self::Int(i)
    }
}

impl From<String> for CellValue {
    fn from(s: String) -> Self {
        Self::Text(s)
    }
}

impl From<&str> for CellValue {
    fn from(s: &str) -> Self {
        Self::Text(s.to_string())
    }
}
