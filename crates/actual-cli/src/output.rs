//! Formatted console and JSON output helpers.

use actual_core::Cents;
use serde::Serialize;

/// Prints a serializable value as formatted JSON to stdout.
pub fn print_json<T: Serialize>(value: &T) {
    let json = serde_json::to_string_pretty(value).expect("serialize JSON");
    println!("{json}");
}

/// Formats cents as a standard dollar string, e.g. "$1,234.56" or "-$45.00".
pub fn format_dollars(cents: Cents) -> String {
    let raw = cents.as_cents();
    let abs = raw.abs();
    let dollars = abs / 100;
    let fraction = abs % 100;

    let sign = if raw < 0 { "-" } else { "" };

    // Format with comma thousands separators
    let s_dollars = dollars.to_string();
    let mut formatted_dollars = String::new();
    let chars: Vec<char> = s_dollars.chars().collect();
    let len = chars.len();

    for (i, ch) in chars.into_iter().enumerate() {
        if i > 0 && (len - i).is_multiple_of(3) {
            formatted_dollars.push(',');
        }
        formatted_dollars.push(ch);
    }

    format!("{sign}${formatted_dollars}.{fraction:02}")
}
