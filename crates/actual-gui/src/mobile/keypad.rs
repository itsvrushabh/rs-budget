//! Mobile numeric keypad and quick transaction logging bottom sheet.
//!
//! Provides optimized touch-first financial keypad entry with integer and decimal modes,
//! arithmetic support, and quick expense capture.

use actual_core::cents::Cents;
use serde::{Deserialize, Serialize};

/// Keypad input button values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeypadKey {
    Digit(u8),
    Decimal,
    Backspace,
    Clear,
    Plus,
    Minus,
    Submit,
}

/// Keypad state machine computing currency amounts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeypadState {
    pub raw_buffer: String,
    pub is_negative: bool,
}

impl Default for KeypadState {
    fn default() -> Self {
        Self {
            raw_buffer: "0".to_string(),
            is_negative: true, // Default to negative for expenses
        }
    }
}

impl KeypadState {
    /// Creates a new keypad state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Handles a keypad button press.
    pub fn press_key(&mut self, key: KeypadKey) {
        match key {
            KeypadKey::Digit(d) => {
                if self.raw_buffer == "0" {
                    self.raw_buffer = d.to_string();
                } else {
                    // Limit to 2 decimal places if decimal point present
                    if let Some(dot_idx) = self.raw_buffer.find('.') {
                        if self.raw_buffer.len() - dot_idx > 2 {
                            return;
                        }
                    }
                    self.raw_buffer.push_str(&d.to_string());
                }
            }
            KeypadKey::Decimal => {
                if !self.raw_buffer.contains('.') {
                    self.raw_buffer.push('.');
                }
            }
            KeypadKey::Backspace => {
                self.raw_buffer.pop();
                if self.raw_buffer.is_empty() {
                    self.raw_buffer = "0".to_string();
                }
            }
            KeypadKey::Clear => {
                self.raw_buffer = "0".to_string();
            }
            KeypadKey::Plus => {
                self.is_negative = false;
            }
            KeypadKey::Minus => {
                self.is_negative = true;
            }
            KeypadKey::Submit => {}
        }
    }

    /// Computes the fixed-point `Cents` value represented by current buffer.
    pub fn to_cents(&self) -> Cents {
        let parsed = self.raw_buffer.parse::<f64>().unwrap_or(0.0);
        let cents_val = (parsed * 100.0).round() as i64;
        let final_cents = if self.is_negative {
            -cents_val.abs()
        } else {
            cents_val.abs()
        };
        Cents(final_cents)
    }

    /// Display string with currency symbol and negative sign.
    pub fn display_string(&self) -> String {
        let sign = if self.is_negative { "-" } else { "+" };
        format!("{sign}${buffer}", sign = sign, buffer = self.raw_buffer)
    }
}

/// Quick expense entry bottom sheet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuickAddSheet {
    pub is_open: bool,
    pub keypad: KeypadState,
    pub selected_account_id: Option<String>,
    pub selected_category_id: Option<String>,
    pub selected_payee_name: String,
    pub notes: String,
}

impl Default for QuickAddSheet {
    fn default() -> Self {
        Self {
            is_open: false,
            keypad: KeypadState::new(),
            selected_account_id: None,
            selected_category_id: None,
            selected_payee_name: String::new(),
            notes: String::new(),
        }
    }
}

impl QuickAddSheet {
    /// Opens sheet with clean state.
    pub fn open(&mut self) {
        self.is_open = true;
        self.keypad = KeypadState::new();
        self.notes.clear();
    }

    /// Closes sheet.
    pub fn close(&mut self) {
        self.is_open = false;
    }
}
