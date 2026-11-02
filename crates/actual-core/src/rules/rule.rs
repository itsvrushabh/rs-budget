//! Rule definition entity and stage enumeration.

use serde::{Deserialize, Serialize};

use crate::rules::action::Action;
use crate::rules::condition::Condition;
use crate::transaction::Transaction;

/// Execution stage of a rule during transaction processing.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
#[serde(rename_all = "snake_case")]
pub enum RuleStage {
    /// Pre-processing stage (e.g. initial payee cleanup, normalizations).
    Pre,
    /// Default processing stage (standard user rules).
    #[default]
    Default,
    /// Post-processing stage (final overrides, validation flags).
    Post,
}

/// Logical operator combining multiple conditions in a rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ConditionsOp {
    #[default]
    And,
    Or,
}

/// A transaction processing rule consisting of conditions and transformation actions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rule {
    /// Unique identifier (UUID).
    pub id: String,
    /// Execution stage.
    pub stage: RuleStage,
    /// Logical combiner for conditions.
    pub conditions_op: ConditionsOp,
    /// List of match conditions.
    pub conditions: Vec<Condition>,
    /// List of actions to execute when conditions match.
    pub actions: Vec<Action>,
    /// Soft-deletion flag.
    pub tombstone: bool,
}

impl Rule {
    /// Creates a new rule.
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        stage: RuleStage,
        conditions_op: ConditionsOp,
        conditions: Vec<Condition>,
        actions: Vec<Action>,
    ) -> Self {
        Self {
            id: id.into(),
            stage,
            conditions_op,
            conditions,
            actions,
            tombstone: false,
        }
    }

    /// Evaluates if this rule matches a transaction.
    #[must_use]
    pub fn matches(&self, tx: &Transaction) -> bool {
        if self.tombstone || self.conditions.is_empty() {
            return false;
        }

        match self.conditions_op {
            ConditionsOp::And => self.conditions.iter().all(|c| c.matches(tx)),
            ConditionsOp::Or => self.conditions.iter().any(|c| c.matches(tx)),
        }
    }

    /// Executes all actions in this rule on the transaction if it matches.
    ///
    /// Returns `true` if the rule matched and actions were applied.
    pub fn apply(&self, tx: &mut Transaction) -> bool {
        if self.matches(tx) {
            for action in &self.actions {
                action.apply(tx);
            }
            true
        } else {
            false
        }
    }
}
