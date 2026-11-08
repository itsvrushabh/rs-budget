//! Multi-currency definitions, exchange rates, and monetary conversion for rs-budget.

use crate::cents::Cents;
use crate::error::DomainError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Standard ISO-4217 currency specification.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Currency {
    /// 3-letter ISO code (e.g., "USD", "EUR", "GBP", "JPY", "INR").
    pub code: String,
    /// Currency symbol (e.g., "$", "€", "£", "¥", "₹").
    pub symbol: String,
    /// Number of decimal digits (usually 2, 0 for JPY, 3 for BHD).
    pub decimal_digits: u8,
}

impl Currency {
    /// Constructs a custom currency specification.
    pub fn new(code: impl Into<String>, symbol: impl Into<String>, decimal_digits: u8) -> Self {
        Self {
            code: code.into().to_uppercase(),
            symbol: symbol.into(),
            decimal_digits,
        }
    }

    /// United States Dollar ($).
    pub fn usd() -> Self {
        Self::new("USD", "$", 2)
    }

    /// Euro (€).
    pub fn eur() -> Self {
        Self::new("EUR", "€", 2)
    }

    /// British Pound Sterling (£).
    pub fn gbp() -> Self {
        Self::new("GBP", "£", 2)
    }

    /// Japanese Yen (¥).
    pub fn jpy() -> Self {
        Self::new("JPY", "¥", 0)
    }

    /// Canadian Dollar (CAD $).
    pub fn cad() -> Self {
        Self::new("CAD", "CA$", 2)
    }

    /// Australian Dollar (AUD $).
    pub fn aud() -> Self {
        Self::new("AUD", "A$", 2)
    }

    /// Indian Rupee (₹).
    pub fn inr() -> Self {
        Self::new("INR", "₹", 2)
    }

    /// Swiss Franc (CHF).
    pub fn chf() -> Self {
        Self::new("CHF", "CHF", 2)
    }

    /// Format a `Cents` value with the currency symbol.
    pub fn format_amount(&self, amount: Cents) -> String {
        let cents_val = amount.as_cents();
        let is_negative = cents_val < 0;
        let abs_val = cents_val.abs();

        if self.decimal_digits == 0 {
            let sign = if is_negative { "-" } else { "" };
            format!("{}{}{}", sign, self.symbol, abs_val)
        } else {
            let divisor = 10_i64.pow(self.decimal_digits as u32);
            let units = abs_val / divisor;
            let fractions = abs_val % divisor;
            let sign = if is_negative { "-" } else { "" };
            format!(
                "{}{}{}.{:0width$}",
                sign,
                self.symbol,
                units,
                fractions,
                width = self.decimal_digits as usize
            )
        }
    }
}

/// Point-in-time exchange rate between two currencies.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExchangeRate {
    /// Base currency code (e.g. "USD").
    pub base_currency: String,
    /// Target/quote currency code (e.g. "EUR").
    pub quote_currency: String,
    /// Exchange multiplier: 1 base = rate * quote.
    pub rate: f64,
    /// Epoch timestamp (seconds) when the rate was recorded.
    pub timestamp: i64,
}

impl ExchangeRate {
    /// Create a new exchange rate entry.
    pub fn new(
        base: impl Into<String>,
        quote: impl Into<String>,
        rate: f64,
        timestamp: i64,
    ) -> Result<Self, DomainError> {
        if rate <= 0.0 || rate.is_nan() || rate.is_infinite() {
            return Err(DomainError::Validation(
                "Exchange rate must be positive and finite".to_string(),
            ));
        }
        Ok(Self {
            base_currency: base.into().to_uppercase(),
            quote_currency: quote.into().to_uppercase(),
            rate,
            timestamp,
        })
    }
}

/// In-memory currency converter and exchange rate registry.
#[derive(Debug, Clone, Default)]
pub struct CurrencyConverter {
    /// Mapping: (Base, Quote) -> ExchangeRate
    rates: HashMap<(String, String), ExchangeRate>,
}

impl CurrencyConverter {
    /// Create a new empty converter.
    pub fn new() -> Self {
        Self {
            rates: HashMap::new(),
        }
    }

    /// Register or update an exchange rate.
    pub fn set_rate(&mut self, rate: ExchangeRate) {
        let key = (rate.base_currency.clone(), rate.quote_currency.clone());
        self.rates.insert(key, rate);
    }

    /// Retrieve an exchange rate between two currency codes.
    pub fn get_rate(&self, from: &str, to: &str) -> Option<f64> {
        let from_code = from.to_uppercase();
        let to_code = to.to_uppercase();

        if from_code == to_code {
            return Some(1.0);
        }

        // Direct rate
        if let Some(r) = self.rates.get(&(from_code.clone(), to_code.clone())) {
            return Some(r.rate);
        }

        // Inverted rate
        if let Some(r) = self.rates.get(&(to_code.clone(), from_code.clone())) {
            if r.rate > 0.0 {
                return Some(1.0 / r.rate);
            }
        }

        None
    }

    /// Convert an amount in `Cents` from one currency to another using the registered rate.
    pub fn convert(&self, amount: Cents, from: &str, to: &str) -> Result<Cents, DomainError> {
        let rate = self.get_rate(from, to).ok_or_else(|| {
            DomainError::Validation(format!(
                "No exchange rate found to convert from {} to {}",
                from, to
            ))
        })?;

        let raw_cents = amount.as_cents();
        let converted_cents = ((raw_cents as f64) * rate).round() as i64;
        Ok(Cents::from_cents(converted_cents))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_currency_formatting() {
        let usd = Currency::usd();
        assert_eq!(usd.format_amount(Cents::from_cents(1550)), "$15.50");
        assert_eq!(usd.format_amount(Cents::from_cents(-450)), "-$4.50");

        let jpy = Currency::jpy();
        assert_eq!(jpy.format_amount(Cents::from_cents(2500)), "¥2500");
    }

    #[test]
    fn test_currency_conversion() {
        let mut converter = CurrencyConverter::new();
        let rate = ExchangeRate::new("USD", "EUR", 0.92, 1700000000).unwrap();
        converter.set_rate(rate);

        // Convert $100.00 USD -> EUR
        let eur = converter.convert(Cents::from_cents(10000), "USD", "EUR").unwrap();
        assert_eq!(eur.as_cents(), 9200); // 92.00 EUR

        // Inverted conversion EUR -> USD
        let usd = converter.convert(Cents::from_cents(9200), "EUR", "USD").unwrap();
        assert_eq!(usd.as_cents(), 10000); // 100.00 USD
    }
}
