//! Rule condition types and evaluation logic.

use chrono::NaiveDate;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::cents::Cents;
use crate::transaction::Transaction;

/// Condition fields on a transaction that can be inspected by rules.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConditionField {
    Payee,
    ImportedPayee,
    Account,
    Category,
    Amount,
    Date,
    Notes,
    Cleared,
    Custom(String),
}

/// Comparison operators for rule conditions.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ConditionOp {
    Is,
    IsNot,
    Contains,
    DoesNotContain,
    Matches,
    OneOf,
    NotOneOf,
    IsApprox,
    Gt,
    Gte,
    Lt,
    Lte,
    Between,
}

/// Dynamic value against which a transaction condition is compared.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ConditionValue {
    Text(String),
    Cents(Cents),
    Number(i64),
    Bool(bool),
    Date(NaiveDate),
    List(Vec<String>),
    Range(Cents, Cents),
}

impl ConditionValue {
    #[must_use]
    pub fn as_text(&self) -> &str {
        match self {
            Self::Text(s) => s.as_str(),
            _ => "",
        }
    }

    #[must_use]
    pub fn as_cents(&self) -> Cents {
        match self {
            Self::Cents(c) => *c,
            Self::Number(n) => Cents::from_cents(*n),
            _ => Cents::ZERO,
        }
    }

    #[must_use]
    pub fn as_bool(&self) -> bool {
        match self {
            Self::Bool(b) => *b,
            _ => false,
        }
    }

    #[must_use]
    pub fn as_list(&self) -> &[String] {
        match self {
            Self::List(l) => l.as_slice(),
            _ => &[],
        }
    }
}

/// A condition rule asserting a property on a transaction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Condition {
    pub field: ConditionField,
    pub op: ConditionOp,
    pub value: ConditionValue,
}

impl Condition {
    /// Creates a new rule condition.
    #[must_use]
    pub fn new(field: ConditionField, op: ConditionOp, value: ConditionValue) -> Self {
        Self { field, op, value }
    }

    /// Evaluates whether this condition matches the given transaction.
    #[must_use]
    pub fn matches(&self, tx: &Transaction) -> bool {
        match self.field {
            ConditionField::Payee => {
                let actual = tx.payee_id.as_deref().unwrap_or("");
                self.matches_text(actual)
            }
            ConditionField::ImportedPayee => {
                let actual = tx.imported_payee.as_deref().unwrap_or("");
                self.matches_text(actual)
            }
            ConditionField::Account => self.matches_text(&tx.account_id),
            ConditionField::Category => {
                let actual = tx.category_id.as_deref().unwrap_or("");
                self.matches_text(actual)
            }
            ConditionField::Notes => {
                let actual = tx.notes.as_deref().unwrap_or("");
                self.matches_text(actual)
            }
            ConditionField::Amount => self.matches_amount(tx.amount),
            ConditionField::Date => self.matches_date(tx.date),
            ConditionField::Cleared => match self.op {
                ConditionOp::Is => tx.cleared == self.value.as_bool(),
                ConditionOp::IsNot => tx.cleared != self.value.as_bool(),
                _ => false,
            },
            ConditionField::Custom(_) => false,
        }
    }

    fn matches_text(&self, actual: &str) -> bool {
        let actual_lower = actual.to_lowercase();
        let target = self.value.as_text();
        let target_lower = target.to_lowercase();

        match self.op {
            ConditionOp::Is => actual_lower == target_lower,
            ConditionOp::IsNot => actual_lower != target_lower,
            ConditionOp::Contains => actual_lower.contains(&target_lower),
            ConditionOp::DoesNotContain => !actual_lower.contains(&target_lower),
            ConditionOp::Matches => Regex::new(target)
                .map(|re| re.is_match(actual))
                .unwrap_or(false),
            ConditionOp::OneOf => self
                .value
                .as_list()
                .iter()
                .any(|item| item.eq_ignore_ascii_case(actual)),
            ConditionOp::NotOneOf => !self
                .value
                .as_list()
                .iter()
                .any(|item| item.eq_ignore_ascii_case(actual)),
            _ => false,
        }
    }

    fn matches_amount(&self, actual: Cents) -> bool {
        let target = self.value.as_cents();

        match self.op {
            ConditionOp::Is => actual == target,
            ConditionOp::IsNot => actual != target,
            ConditionOp::Gt => actual > target,
            ConditionOp::Gte => actual >= target,
            ConditionOp::Lt => actual < target,
            ConditionOp::Lte => actual <= target,
            ConditionOp::Between => match &self.value {
                ConditionValue::Range(min, max) => actual >= *min && actual <= *max,
                _ => false,
            },
            ConditionOp::IsApprox => {
                // Approximate matching: within 5% or 250 cents ($2.50)
                let diff = (actual - target).abs();
                let tolerance_pct = (target.as_cents().abs() * 5) / 100;
                let tolerance = Cents::from_cents(tolerance_pct.max(250));
                diff <= tolerance
            }
            _ => false,
        }
    }

    fn matches_date(&self, actual: NaiveDate) -> bool {
        match &self.value {
            ConditionValue::Date(target) => match self.op {
                ConditionOp::Is => actual == *target,
                ConditionOp::IsNot => actual != *target,
                ConditionOp::Gt => actual > *target,
                ConditionOp::Gte => actual >= *target,
                ConditionOp::Lt => actual < *target,
                ConditionOp::Lte => actual <= *target,
                _ => false,
            },
            _ => false,
        }
    }
}
