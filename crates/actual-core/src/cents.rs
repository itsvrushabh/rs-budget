//! Monetary precision representation using fixed-point integer cents (`Cents`).

use serde::{Deserialize, Serialize};
use std::fmt;
use std::iter::Sum;
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};
use std::str::FromStr;

use crate::error::DomainError;

/// Maximum safe integer for monetary calculations matching upstream Actual Budget (`2^51 - 1`).
pub const MAX_SAFE_CENTS: i64 = (1i64 << 51) - 1;
pub const MIN_SAFE_CENTS: i64 = -MAX_SAFE_CENTS;

/// Fixed-point monetary precision type representing financial values in integer cents (e.g. 1050 = $10.50).
///
/// Disallows floating-point storage to guarantee exact financial arithmetic with zero rounding errors.
#[derive(
    Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Cents(pub i64);

impl Cents {
    /// Zero cents ($0.00).
    pub const ZERO: Self = Self(0);

    /// One major currency unit / dollar (100 cents = $1.00).
    pub const ONE: Self = Self(100);

    /// Maximum safe monetary value.
    pub const MAX: Self = Self(MAX_SAFE_CENTS);

    /// Minimum safe monetary value.
    pub const MIN: Self = Self(MIN_SAFE_CENTS);

    /// Creates a new `Cents` value after asserting that it is within safe calculation limits.
    pub fn new(val: i64) -> Result<Self, DomainError> {
        if !(MIN_SAFE_CENTS..=MAX_SAFE_CENTS).contains(&val) {
            return Err(DomainError::CentsOverflow);
        }
        Ok(Self(val))
    }

    /// Creates a `Cents` value directly without boundary checks.
    #[must_use]
    pub const fn from_cents(val: i64) -> Self {
        Self(val)
    }

    /// Constructs a `Cents` instance from major currency units (e.g. whole dollars).
    pub fn from_major(major: i64) -> Result<Self, DomainError> {
        Self::from_major_and_minor(major, 0)
    }

    /// Constructs a `Cents` instance from whole major units (dollars) and minor units (cents).
    pub fn from_major_and_minor(major: i64, minor: i64) -> Result<Self, DomainError> {
        let total = major
            .checked_mul(100)
            .and_then(|m| m.checked_add(minor))
            .ok_or(DomainError::CentsOverflow)?;
        Self::new(total)
    }

    /// Returns the raw integer cent value.
    #[must_use]
    pub const fn as_cents(self) -> i64 {
        self.0
    }

    /// Returns whether this monetary value is exactly zero.
    #[must_use]
    pub const fn is_zero(self) -> bool {
        self.0 == 0
    }

    /// Returns whether this monetary value is strictly positive (> 0).
    #[must_use]
    pub const fn is_positive(self) -> bool {
        self.0 > 0
    }

    /// Returns whether this monetary value is strictly negative (< 0).
    #[must_use]
    pub const fn is_negative(self) -> bool {
        self.0 < 0
    }

    /// Returns the absolute value.
    #[must_use]
    pub const fn abs(self) -> Self {
        Self(self.0.abs())
    }

    /// Checked addition.
    #[must_use]
    pub fn checked_add(self, rhs: Self) -> Option<Self> {
        self.0.checked_add(rhs.0).and_then(|val| {
            if (MIN_SAFE_CENTS..=MAX_SAFE_CENTS).contains(&val) {
                Some(Self(val))
            } else {
                None
            }
        })
    }

    /// Checked subtraction.
    #[must_use]
    pub fn checked_sub(self, rhs: Self) -> Option<Self> {
        self.0.checked_sub(rhs.0).and_then(|val| {
            if (MIN_SAFE_CENTS..=MAX_SAFE_CENTS).contains(&val) {
                Some(Self(val))
            } else {
                None
            }
        })
    }

    /// Checked multiplication by an integer scalar.
    #[must_use]
    pub fn checked_mul(self, rhs: i64) -> Option<Self> {
        self.0.checked_mul(rhs).and_then(|val| {
            if (MIN_SAFE_CENTS..=MAX_SAFE_CENTS).contains(&val) {
                Some(Self(val))
            } else {
                None
            }
        })
    }

    /// Checked integer division.
    #[must_use]
    pub fn checked_div(self, rhs: i64) -> Option<Self> {
        if rhs == 0 {
            return None;
        }
        self.0.checked_div(rhs).map(Self)
    }

    /// Formats the amount into a decimal string with the given number of decimal places (default 2).
    #[must_use]
    pub fn to_decimal_string(self, decimal_places: usize) -> String {
        let is_negative = self.0 < 0;
        let abs_val = self.0.abs();

        let divisor = 10i64.pow(decimal_places as u32);
        let major = abs_val / divisor;
        let minor = abs_val % divisor;

        let sign = if is_negative { "-" } else { "" };
        format!("{sign}{major}.{minor:0width$}", width = decimal_places)
    }
}

impl fmt::Display for Cents {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_decimal_string(2))
    }
}

impl FromStr for Cents {
    type Err = DomainError;

    /// Parses a string representation of currency into `Cents`.
    ///
    /// Accepts standard formats such as `"12.34"`, `"$12.34"`, `"-12.34"`, `"-$12.34"`,
    /// `"($12.34)"` (accounting parentheses for negative), and integer strings `"1234"`.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Err(DomainError::CentsParse(s.to_string()));
        }

        let mut cleaned = trimmed.replace(['$', ',', ' '], "");
        let mut is_negative = false;

        // Check for accounting parentheses e.g. (10.50)
        if cleaned.starts_with('(') && cleaned.ends_with(')') {
            is_negative = true;
            cleaned = cleaned[1..cleaned.len() - 1].to_string();
        }

        if let Some(rest) = cleaned.strip_prefix('-') {
            is_negative = true;
            cleaned = rest.to_string();
        } else if let Some(rest) = cleaned.strip_prefix('+') {
            cleaned = rest.to_string();
        }

        if cleaned.is_empty() {
            return Err(DomainError::CentsParse(s.to_string()));
        }

        let total_cents = if let Some(dot_idx) = cleaned.find('.') {
            let (major_str, minor_raw) = cleaned.split_at(dot_idx);
            let minor_str = &minor_raw[1..]; // skip '.'

            let major = if major_str.is_empty() {
                0i64
            } else {
                major_str
                    .parse::<i64>()
                    .map_err(|_| DomainError::CentsParse(s.to_string()))?
            };

            // Format minor cents to exactly 2 digits
            let minor = match minor_str.len() {
                0 => 0i64,
                1 => {
                    minor_str
                        .parse::<i64>()
                        .map_err(|_| DomainError::CentsParse(s.to_string()))?
                        * 10
                }
                2 => minor_str
                    .parse::<i64>()
                    .map_err(|_| DomainError::CentsParse(s.to_string()))?,
                _ => {
                    // Truncate/round to 2 decimals
                    let truncated = &minor_str[..2];
                    truncated
                        .parse::<i64>()
                        .map_err(|_| DomainError::CentsParse(s.to_string()))?
                }
            };

            major
                .checked_mul(100)
                .and_then(|m| m.checked_add(minor))
                .ok_or(DomainError::CentsOverflow)?
        } else {
            cleaned
                .parse::<i64>()
                .map_err(|_| DomainError::CentsParse(s.to_string()))?
        };

        let final_val = if is_negative {
            -total_cents
        } else {
            total_cents
        };
        Self::new(final_val)
    }
}

impl Add for Cents {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        self.checked_add(rhs)
            .expect("monetary addition overflowed safe bounds")
    }
}

impl AddAssign for Cents {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sub for Cents {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        self.checked_sub(rhs)
            .expect("monetary subtraction overflowed safe bounds")
    }
}

impl SubAssign for Cents {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl Mul<i64> for Cents {
    type Output = Self;
    fn mul(self, rhs: i64) -> Self::Output {
        self.checked_mul(rhs)
            .expect("monetary scalar multiplication overflowed safe bounds")
    }
}

impl MulAssign<i64> for Cents {
    fn mul_assign(&mut self, rhs: i64) {
        *self = *self * rhs;
    }
}

impl Div<i64> for Cents {
    type Output = Self;
    fn div(self, rhs: i64) -> Self::Output {
        self.checked_div(rhs)
            .expect("monetary scalar division by zero or overflow")
    }
}

impl DivAssign<i64> for Cents {
    fn div_assign(&mut self, rhs: i64) {
        *self = *self / rhs;
    }
}

impl Neg for Cents {
    type Output = Self;
    fn neg(self) -> Self::Output {
        Self(-self.0)
    }
}

impl Sum for Cents {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::ZERO, |acc, x| acc + x)
    }
}

impl<'a> Sum<&'a Cents> for Cents {
    fn sum<I: Iterator<Item = &'a Self>>(iter: I) -> Self {
        iter.fold(Self::ZERO, |acc, &x| acc + x)
    }
}
