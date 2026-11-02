use serde::{Deserialize, Serialize};

/// Monetary amount represented in fixed-point integer cents (e.g. 1000 = $10.00).
pub type Cents = i64;

/// Account entity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub name: String,
    pub offbudget: bool,
    pub closed: bool,
}
