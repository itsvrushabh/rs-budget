//! Integration tests for `actual-cli` command-line workflows.

use std::process::Command;

fn run_cli(args: &[&str]) -> (bool, String, String) {
    let bin = env!("CARGO_BIN_EXE_actual");
    let output = Command::new(bin)
        .args(args)
        .output()
        .expect("failed to execute actual binary");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (output.status.success(), stdout, stderr)
}

#[test]
fn test_cli_help() {
    let (success, stdout, _) = run_cli(&["--help"]);
    assert!(success);
    assert!(stdout.contains("actual"));
    assert!(stdout.contains("budget"));
    assert!(stdout.contains("account"));
    assert!(stdout.contains("transaction"));
    assert!(stdout.contains("import"));
    assert!(stdout.contains("export"));
}

#[test]
fn test_account_create_and_list_json() {
    // Run with persistent temp sqlite db
    let temp_dir = std::env::temp_dir();
    let db_path = temp_dir.join(format!("test_acc_{}.sqlite", uuid::Uuid::new_v4()));
    let db_str = db_path.to_str().unwrap();

    // 1. Create account
    let (ok, out, _) = run_cli(&[
        "--db",
        db_str,
        "--json",
        "account",
        "create",
        "--name",
        "Test Checking",
    ]);
    assert!(ok);
    let create_res: serde_json::Value = serde_json::from_str(&out).expect("parse json");
    assert_eq!(create_res["status"], "ok");
    assert_eq!(create_res["name"], "Test Checking");

    // 2. List accounts
    let (ok, out, _) = run_cli(&["--db", db_str, "--json", "account", "list"]);
    assert!(ok);
    let list_res: serde_json::Value = serde_json::from_str(&out).expect("parse json");
    assert!(list_res.is_array());
    assert_eq!(list_res.as_array().unwrap().len(), 1);
    assert_eq!(list_res[0]["name"], "Test Checking");

    // Cleanup
    let _ = std::fs::remove_file(&db_path);
}

#[test]
fn test_budget_inspect_json() {
    let (ok, out, _) = run_cli(&[
        "--in-memory",
        "--json",
        "budget",
        "inspect",
        "--month",
        "2024-06",
    ]);
    assert!(ok);
    let inspect_res: serde_json::Value = serde_json::from_str(&out).expect("parse json");
    assert_eq!(inspect_res["month"], "2024-06");
    assert!(inspect_res["to_budget"].is_string());
}

#[test]
fn test_transaction_add_and_list() {
    let temp_dir = std::env::temp_dir();
    let db_path = temp_dir.join(format!("test_tx_{}.sqlite", uuid::Uuid::new_v4()));
    let db_str = db_path.to_str().unwrap();

    // Create account
    let (ok, out, _) = run_cli(&[
        "--db", db_str, "--json", "account", "create", "--name", "Savings",
    ]);
    assert!(ok);
    let acct: serde_json::Value = serde_json::from_str(&out).unwrap();
    let acct_id = acct["id"].as_str().unwrap();

    // Add transaction
    let (ok, out, err) = run_cli(&[
        "--db",
        db_str,
        "--json",
        "transaction",
        "add",
        "--account",
        acct_id,
        "--amount",
        "-45.50",
        "--date",
        "2024-06-15",
        "--notes",
        "Grocery run",
    ]);
    assert!(ok, "Transaction add failed: {err}");
    let tx_res: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(tx_res["status"], "ok");

    // List transactions
    let (ok, out, _) = run_cli(&[
        "--db",
        db_str,
        "--json",
        "transaction",
        "list",
        "--account",
        acct_id,
    ]);
    assert!(ok);
    let list: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(list.as_array().unwrap().len(), 1);
    assert_eq!(list[0]["amount"], "-$45.50");

    let _ = std::fs::remove_file(&db_path);
}

#[test]
fn test_import_and_export_cli() {
    let temp_dir = std::env::temp_dir();
    let db_path = temp_dir.join(format!("test_imp_{}.sqlite", uuid::Uuid::new_v4()));
    let db_str = db_path.to_str().unwrap();

    // Create account
    let (ok, out, _) = run_cli(&[
        "--db", db_str, "--json", "account", "create", "--name", "Checking",
    ]);
    assert!(ok);
    let acct: serde_json::Value = serde_json::from_str(&out).unwrap();
    let acct_id = acct["id"].as_str().unwrap();

    // Import sample.qif fixture
    let qif_path = "../../fixtures/imports/sample.qif";
    let (ok, out, _) = run_cli(&[
        "--db",
        db_str,
        "--json",
        "import",
        "--account",
        acct_id,
        "--file",
        qif_path,
    ]);
    assert!(ok);
    let imp_res: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(imp_res["status"], "ok");
    assert_eq!(imp_res["total_parsed"], 3);
    assert_eq!(imp_res["imported"], 3);

    // Export to JSON
    let (ok, out, _) = run_cli(&[
        "--db",
        db_str,
        "export",
        "--format",
        "json",
        "--account",
        acct_id,
    ]);
    assert!(ok);
    let exp: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(exp.as_array().unwrap().len(), 3);

    // Export to CSV
    let (ok, out, _) = run_cli(&[
        "--db",
        db_str,
        "export",
        "--format",
        "csv",
        "--account",
        acct_id,
    ]);
    assert!(ok);
    assert!(out.contains("id,date,account_id,amount_cents"));

    let _ = std::fs::remove_file(&db_path);
}

#[test]
fn test_error_exit_status() {
    // Try to add transaction with nonexistent account
    let (ok, _, stderr) = run_cli(&[
        "--in-memory",
        "transaction",
        "add",
        "--account",
        "nonexistent-id",
        "--amount",
        "10.00",
    ]);
    assert!(!ok);
    assert!(stderr.contains("Error"));
}
