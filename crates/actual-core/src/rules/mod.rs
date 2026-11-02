//! Transaction rules engine for automated categorization, splitting, and normalization.

pub mod action;
pub mod condition;
pub mod engine;
pub mod rule;

pub use action::{Action, ActionField, ActionOp, ActionValue, SplitAllocation};
pub use condition::{Condition, ConditionField, ConditionOp, ConditionValue};
pub use engine::RulesEngine;
pub use rule::{ConditionsOp, Rule, RuleStage};
