use chrono::NaiveDate;

use actual_core::cents::Cents;
use actual_core::rules::{
    Action, ActionField, ActionOp, ActionValue, Condition, ConditionField, ConditionOp,
    ConditionValue, ConditionsOp, Rule, RuleStage, RulesEngine, SplitAllocation,
};
use actual_core::transaction::Transaction;

fn dollars(d: i64) -> Cents {
    Cents::from_cents(d * 100)
}

#[test]
fn test_rule_condition_payee_matching() {
    let mut tx = Transaction::new(
        "tx_1",
        "acct_checking",
        NaiveDate::from_ymd_opt(2026, 11, 2).unwrap(),
        dollars(-45),
    );
    tx.imported_payee = Some("AMZN MKTP US*123456".to_string());
    tx.payee_id = Some("Amazon".to_string());

    // 1. Contains (case-insensitive)
    let cond_contains = Condition::new(
        ConditionField::ImportedPayee,
        ConditionOp::Contains,
        ConditionValue::Text("amzn".to_string()),
    );
    assert!(cond_contains.matches(&tx));

    // 2. Regex matches
    let cond_regex = Condition::new(
        ConditionField::ImportedPayee,
        ConditionOp::Matches,
        ConditionValue::Text(r"^AMZN MKTP.*".to_string()),
    );
    assert!(cond_regex.matches(&tx));

    // 3. OneOf
    let cond_one_of = Condition::new(
        ConditionField::Payee,
        ConditionOp::OneOf,
        ConditionValue::List(vec!["Target".to_string(), "Amazon".to_string()]),
    );
    assert!(cond_one_of.matches(&tx));

    // 4. DoesNotContain
    let cond_not_contain = Condition::new(
        ConditionField::Payee,
        ConditionOp::DoesNotContain,
        ConditionValue::Text("Walmart".to_string()),
    );
    assert!(cond_not_contain.matches(&tx));
}

#[test]
fn test_rule_condition_amount_and_range() {
    let tx = Transaction::new(
        "tx_2",
        "acct_checking",
        NaiveDate::from_ymd_opt(2026, 11, 2).unwrap(),
        dollars(-120),
    );

    // Gt / Lt (remember negative numbers: -120 < -100)
    let cond_lt = Condition::new(
        ConditionField::Amount,
        ConditionOp::Lt,
        ConditionValue::Cents(dollars(-100)),
    );
    assert!(cond_lt.matches(&tx));

    // Between
    let cond_between = Condition::new(
        ConditionField::Amount,
        ConditionOp::Between,
        ConditionValue::Range(dollars(-150), dollars(-100)),
    );
    assert!(cond_between.matches(&tx));

    // IsApprox
    let tx_approx = Transaction::new(
        "tx_approx",
        "acct_checking",
        NaiveDate::from_ymd_opt(2026, 11, 2).unwrap(),
        Cents::from_cents(-10150), // -$101.50
    );
    let cond_approx = Condition::new(
        ConditionField::Amount,
        ConditionOp::IsApprox,
        ConditionValue::Cents(dollars(-100)),
    );
    assert!(cond_approx.matches(&tx_approx));
}

#[test]
fn test_rule_action_auto_categorize_and_notes() {
    let mut tx = Transaction::new(
        "tx_3",
        "acct_checking",
        NaiveDate::from_ymd_opt(2026, 11, 2).unwrap(),
        dollars(-15),
    );
    tx.imported_payee = Some("STARBUCKS #1234 COFFEE".to_string());

    let rule = Rule::new(
        "rule_coffee",
        RuleStage::Default,
        ConditionsOp::And,
        vec![Condition::new(
            ConditionField::ImportedPayee,
            ConditionOp::Contains,
            ConditionValue::Text("starbucks".to_string()),
        )],
        vec![
            Action::new(
                ActionField::Category,
                ActionOp::Set,
                ActionValue::Text("cat_coffee".to_string()),
            ),
            Action::new(
                ActionField::Payee,
                ActionOp::Set,
                ActionValue::Text("payee_starbucks".to_string()),
            ),
            Action::new(
                ActionField::Notes,
                ActionOp::AppendNotes,
                ActionValue::Text("[AutoCategorized]".to_string()),
            ),
            Action::new(ActionField::Cleared, ActionOp::Set, ActionValue::Bool(true)),
        ],
    );

    assert!(rule.apply(&mut tx));
    assert_eq!(tx.category_id.as_deref(), Some("cat_coffee"));
    assert_eq!(tx.payee_id.as_deref(), Some("payee_starbucks"));
    assert_eq!(tx.notes.as_deref(), Some("[AutoCategorized]"));
    assert!(tx.cleared);
}

#[test]
fn test_rule_action_auto_split() {
    let mut tx = Transaction::new(
        "tx_paycheck",
        "acct_checking",
        NaiveDate::from_ymd_opt(2026, 11, 1).unwrap(),
        dollars(3000),
    );
    tx.imported_payee = Some("ACME CORP PAYROLL".to_string());

    let rule_split = Rule::new(
        "rule_payroll_split",
        RuleStage::Default,
        ConditionsOp::And,
        vec![Condition::new(
            ConditionField::ImportedPayee,
            ConditionOp::Contains,
            ConditionValue::Text("payroll".to_string()),
        )],
        vec![Action::new(
            ActionField::Split,
            ActionOp::SetSplit,
            ActionValue::Splits(vec![
                SplitAllocation {
                    amount: dollars(2500),
                    category_id: Some("cat_salary".to_string()),
                    payee_id: None,
                    notes: Some("Base Salary".to_string()),
                },
                SplitAllocation {
                    amount: dollars(500),
                    category_id: Some("cat_bonus".to_string()),
                    payee_id: None,
                    notes: Some("Performance Bonus".to_string()),
                },
            ]),
        )],
    );

    assert!(rule_split.apply(&mut tx));
    assert!(tx.is_split_parent());
    assert_eq!(tx.subtransactions.len(), 2);
    assert_eq!(tx.subtransactions[0].amount, dollars(2500));
    assert_eq!(
        tx.subtransactions[0].category_id.as_deref(),
        Some("cat_salary")
    );
    assert_eq!(tx.subtransactions[1].amount, dollars(500));
    assert_eq!(
        tx.subtransactions[1].category_id.as_deref(),
        Some("cat_bonus")
    );
    assert!(tx.validate_split_balance().is_ok());
}

#[test]
fn test_rules_engine_stage_ordering() {
    let mut engine = RulesEngine::new();

    // Add Post rule first
    engine.add_rule(Rule::new(
        "rule_post",
        RuleStage::Post,
        ConditionsOp::And,
        vec![Condition::new(
            ConditionField::Account,
            ConditionOp::Is,
            ConditionValue::Text("acct_checking".to_string()),
        )],
        vec![Action::new(
            ActionField::Notes,
            ActionOp::AppendNotes,
            ActionValue::Text("POST".to_string()),
        )],
    ));

    // Add Pre rule second
    engine.add_rule(Rule::new(
        "rule_pre",
        RuleStage::Pre,
        ConditionsOp::And,
        vec![Condition::new(
            ConditionField::Account,
            ConditionOp::Is,
            ConditionValue::Text("acct_checking".to_string()),
        )],
        vec![Action::new(
            ActionField::Notes,
            ActionOp::Set,
            ActionValue::Text("PRE".to_string()),
        )],
    ));

    // Add Default rule third
    engine.add_rule(Rule::new(
        "rule_default",
        RuleStage::Default,
        ConditionsOp::And,
        vec![Condition::new(
            ConditionField::Account,
            ConditionOp::Is,
            ConditionValue::Text("acct_checking".to_string()),
        )],
        vec![Action::new(
            ActionField::Notes,
            ActionOp::AppendNotes,
            ActionValue::Text("DEFAULT".to_string()),
        )],
    ));

    let mut tx = Transaction::new(
        "tx_stage",
        "acct_checking",
        NaiveDate::from_ymd_opt(2026, 11, 2).unwrap(),
        dollars(-50),
    );

    let applied = engine.apply(&mut tx);
    assert_eq!(applied, 3);
    // PRE sets "PRE", DEFAULT appends "DEFAULT", POST appends "POST"
    assert_eq!(tx.notes.as_deref(), Some("PRE DEFAULT POST"));
}

#[test]
fn test_rule_json_serde_roundtrip() {
    let rule = Rule::new(
        "rule_serde",
        RuleStage::Default,
        ConditionsOp::Or,
        vec![
            Condition::new(
                ConditionField::Payee,
                ConditionOp::Is,
                ConditionValue::Text("Grocery Store".to_string()),
            ),
            Condition::new(
                ConditionField::Amount,
                ConditionOp::Gt,
                ConditionValue::Cents(dollars(1000)),
            ),
        ],
        vec![Action::new(
            ActionField::Cleared,
            ActionOp::Set,
            ActionValue::Bool(true),
        )],
    );

    let json = serde_json::to_string(&rule).unwrap();
    let deserialized: Rule = serde_json::from_str(&json).unwrap();
    assert_eq!(rule, deserialized);
}
