//! Rules evaluation engine coordinating stage-based execution across transaction lifecycles.

use crate::rules::rule::{Rule, RuleStage};
use crate::transaction::Transaction;

/// Rules engine maintaining and executing rules deterministically across pipeline stages.
#[derive(Debug, Clone, Default)]
pub struct RulesEngine {
    rules: Vec<Rule>,
}

impl RulesEngine {
    /// Creates a new empty `RulesEngine`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a rule to the engine.
    pub fn add_rule(&mut self, rule: Rule) {
        self.rules.push(rule);
        self.sort_rules();
    }

    /// Sets the full list of rules, replacing existing ones.
    pub fn set_rules(&mut self, rules: Vec<Rule>) {
        self.rules = rules;
        self.sort_rules();
    }

    /// Sorts rules deterministically by execution stage (`Pre` -> `Default` -> `Post`).
    fn sort_rules(&mut self) {
        self.rules.sort_by_key(|r| r.stage);
    }

    /// Returns a slice of all active rules.
    #[must_use]
    pub fn rules(&self) -> &[Rule] {
        &self.rules
    }

    /// Runs all matching rules against a transaction in stage order (`Pre` -> `Default` -> `Post`).
    ///
    /// Returns the number of rules that matched and applied actions.
    pub fn apply(&self, tx: &mut Transaction) -> usize {
        let mut applied_count = 0;

        for rule in &self.rules {
            if rule.apply(tx) {
                applied_count += 1;
            }
        }

        applied_count
    }

    /// Runs only rules belonging to a specific execution stage.
    pub fn apply_stage(&self, tx: &mut Transaction, stage: RuleStage) -> usize {
        let mut applied_count = 0;

        for rule in self.rules.iter().filter(|r| r.stage == stage) {
            if rule.apply(tx) {
                applied_count += 1;
            }
        }

        applied_count
    }
}
