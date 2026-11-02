use actual_storage::{
    check_database_validity, get_applied_migrations, get_available_migrations,
    patch_bad_migrations, SqliteStore, StorageError, ADDITIVE_ONLY_CUTOFF,
};
use rusqlite::Connection;

#[test]
fn test_in_memory_initialization_and_migration() {
    let store = SqliteStore::open_in_memory().expect("Failed to open in-memory store");

    let applied = store
        .applied_migrations()
        .expect("Failed to fetch applied migrations");
    let available = get_available_migrations();

    assert_eq!(
        applied.len(),
        available.len(),
        "All available migrations should be applied"
    );
    assert_eq!(applied.len(), 60, "Expected exactly 60 migrations");

    // Verify key tables exist
    let conn = store.conn();
    let tables: Vec<String> = {
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name ASC")
            .unwrap();
        stmt.query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(Result::ok)
            .collect()
    };

    assert!(tables.contains(&"accounts".to_string()));
    assert!(tables.contains(&"transactions".to_string()));
    assert!(tables.contains(&"categories".to_string()));
    assert!(tables.contains(&"category_groups".to_string()));
    assert!(tables.contains(&"dashboard".to_string()));
    assert!(tables.contains(&"dashboard_pages".to_string()));
    assert!(tables.contains(&"preferences".to_string()));
    assert!(tables.contains(&"messages_pending".to_string()));
    assert!(tables.contains(&"zero_budgets".to_string()));
    assert!(tables.contains(&"reflect_budgets".to_string()));
    assert!(tables.contains(&"notes".to_string()));
    assert!(tables.contains(&"kvcache".to_string()));
    assert!(tables.contains(&"__migrations__".to_string()));
    assert!(tables.contains(&"__meta__".to_string()));

    // Verify legacy spreadsheet_cells table was dropped
    assert!(!tables.contains(&"spreadsheet_cells".to_string()));

    // Verify default dashboard page and widgets
    let pages_count: i64 = conn
        .query_row(
            "SELECT count(*) FROM dashboard_pages WHERE name = 'Main'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(pages_count, 1);

    let default_page_id: String = conn
        .query_row(
            "SELECT id FROM dashboard_pages WHERE name = 'Main'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    let widgets_count: i64 = conn
        .query_row(
            "SELECT count(*) FROM dashboard WHERE dashboard_page_id = ?1",
            [&default_page_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(widgets_count, 12, "Expected 12 default dashboard widgets");
}

#[test]
fn test_transaction_commit_and_rollback() {
    let mut store = SqliteStore::open_in_memory().expect("Store should open");

    // Commit test
    store
        .with_transaction(|tx| {
            tx.execute(
                "INSERT INTO accounts (id, name, offbudget, closed) VALUES (?1, ?2, 0, 0)",
                rusqlite::params!["acc-1", "Checking Account"],
            )?;
            Ok(())
        })
        .expect("Transaction should succeed");

    let count: i64 = store
        .conn()
        .query_row(
            "SELECT count(*) FROM accounts WHERE id = 'acc-1'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);

    // Rollback test
    let res: Result<(), StorageError> = store.with_transaction(|tx| {
        tx.execute(
            "INSERT INTO accounts (id, name, offbudget, closed) VALUES (?1, ?2, 0, 0)",
            rusqlite::params!["acc-2", "Savings Account"],
        )?;
        Err(StorageError::Transaction("Force rollback".to_string()))
    });

    assert!(res.is_err());
    let count: i64 = store
        .conn()
        .query_row(
            "SELECT count(*) FROM accounts WHERE id = 'acc-2'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        count, 0,
        "Rolled back transaction must not persist account acc-2"
    );
}

#[test]
fn test_bad_migration_patch() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        r#"
        CREATE TABLE __migrations__ (id INT PRIMARY KEY NOT NULL);
        INSERT INTO __migrations__ (id) VALUES (1685375406832);
        "#,
    )
    .unwrap();

    patch_bad_migrations(&conn).expect("Patch should succeed");

    let ids = get_applied_migrations(&conn).unwrap();
    assert!(!ids.contains(&1685375406832));
    assert!(ids.contains(&1688749527273));
}

#[test]
fn test_database_validity_prefix_and_tolerance() {
    let available: Vec<i64> = get_available_migrations().iter().map(|m| m.id).collect();

    // 1. Exact match passes
    assert!(check_database_validity(&available, &available).is_ok());

    // 2. Ordered prefix passes
    let prefix = available[..10].to_vec();
    assert!(check_database_validity(&prefix, &available).is_ok());

    // 3. Unknown migration prior to cutoff fails
    let mut unknown_old = available.clone();
    unknown_old.push(1000); // <= ADDITIVE_ONLY_CUTOFF
    let err = check_database_validity(&unknown_old, &available);
    assert!(err.is_err());

    // 4. Unknown migration AFTER cutoff with all known available present is tolerated
    let mut unknown_new = available.clone();
    let future_id = ADDITIVE_ONLY_CUTOFF + 100_000;
    unknown_new.push(future_id);
    assert!(check_database_validity(&unknown_new, &available).is_ok());

    // 5. Gap in pre-cutoff migrations fails
    let mut gap_prefix = available[..10].to_vec();
    gap_prefix.remove(3);
    assert!(check_database_validity(&gap_prefix, &available).is_err());

    // 6. Unknown future migration when known migrations are incomplete fails
    let mut incomplete_with_future = available[..5].to_vec();
    incomplete_with_future.push(future_id);
    assert!(check_database_validity(&incomplete_with_future, &available).is_err());
}

#[test]
fn test_disk_store_wal_and_pragmas() {
    let tmp_dir = tempfile::tempdir().expect("Create temp dir");
    let db_path = tmp_dir.path().join("test_budget.sqlite");

    let store = SqliteStore::open(&db_path).expect("Open disk store");
    let conn = store.conn();

    let journal_mode: String = conn
        .query_row("PRAGMA journal_mode;", [], |r| r.get(0))
        .unwrap();
    assert_eq!(journal_mode.to_lowercase(), "wal");

    let foreign_keys: i64 = conn
        .query_row("PRAGMA foreign_keys;", [], |r| r.get(0))
        .unwrap();
    assert_eq!(foreign_keys, 1);

    let synchronous: i64 = conn
        .query_row("PRAGMA synchronous;", [], |r| r.get(0))
        .unwrap();
    assert_eq!(synchronous, 1); // 1 = NORMAL
}

#[test]
fn test_legacy_spreadsheet_cells_migration() {
    let mut conn = Connection::open_in_memory().unwrap();
    actual_storage::initialize_database(&mut conn).unwrap();

    // Apply migrations up to migration 1632571489012 (when spreadsheet_cells was migrated)
    let available = get_available_migrations();
    let target_idx = available
        .iter()
        .position(|m| m.id == 1632571489012)
        .expect("Migration 1632571489012 must exist");

    for m in &available[..target_idx] {
        match m.handler {
            actual_storage::MigrationHandler::Sql(sql) => conn.execute_batch(sql).unwrap(),
            actual_storage::MigrationHandler::Custom(f) => f(&conn).unwrap(),
        }
        conn.execute("INSERT INTO __migrations__ (id) VALUES (?1)", [m.id])
            .unwrap();
    }

    // Insert legacy spreadsheet_cells data as it existed before migration 1632571489012
    conn.execute_batch(
        r#"
        INSERT INTO spreadsheet_cells (name, expr, cachedValue) VALUES
            ('budget202401!budget-cat1', '500', NULL),
            ('budget202401!carryover-cat1', 'true', NULL),
            ('budget-report202402!budget-cat2', '1200', NULL),
            ('budget202401!buffered', '300', NULL),
            ('notes!acc-notes', NULL, '"Important bank note"');
        "#,
    )
    .unwrap();

    // Run remaining migrations (which applies 1632571489012 and subsequent migrations)
    actual_storage::migrate(&mut conn).expect("Migrate should succeed");

    // Verify zero_budgets populated
    let amount: i64 = conn
        .query_row(
            "SELECT amount FROM zero_budgets WHERE id = '2024-01-cat1'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let carryover: i64 = conn
        .query_row(
            "SELECT carryover FROM zero_budgets WHERE id = '2024-01-cat1'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(amount, 500);
    assert_eq!(carryover, 1);

    // Verify reflect_budgets populated
    let report_amount: i64 = conn
        .query_row(
            "SELECT amount FROM reflect_budgets WHERE id = '2024-02-cat2'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(report_amount, 1200);

    // Verify zero_budget_months populated
    let buffered: i64 = conn
        .query_row(
            "SELECT buffered FROM zero_budget_months WHERE id = '2024-01'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(buffered, 300);

    // Verify notes populated
    let note: String = conn
        .query_row("SELECT note FROM notes WHERE id = 'acc-notes'", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(note, "Important bank note");
}

#[test]
fn test_legacy_custom_reports_migration() {
    let mut conn = Connection::open_in_memory().unwrap();
    actual_storage::initialize_database(&mut conn).unwrap();

    let available = get_available_migrations();
    let target_idx = available
        .iter()
        .position(|m| m.id == 1722717601000)
        .expect("Migration 1722717601000 must exist");

    for m in &available[..target_idx] {
        match m.handler {
            actual_storage::MigrationHandler::Sql(sql) => conn.execute_batch(sql).unwrap(),
            actual_storage::MigrationHandler::Custom(f) => f(&conn).unwrap(),
        }
        conn.execute("INSERT INTO __migrations__ (id) VALUES (?1)", [m.id])
            .unwrap();
    }

    // Insert active categories
    conn.execute(
        "INSERT INTO categories (id, name, tombstone) VALUES ('cat-a', 'Groceries', 0)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO categories (id, name, tombstone) VALUES ('cat-b', 'Dining', 0)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO categories (id, name, tombstone) VALUES ('cat-c', 'Utilities', 0)",
        [],
    )
    .unwrap();

    // Insert a custom report with selected_categories containing only cat-a and cat-b (subset)
    conn.execute(
        "INSERT INTO custom_reports (id, name, selected_categories, conditions, tombstone) VALUES (?1, ?2, ?3, ?4, 0)",
        rusqlite::params![
            "rep-1",
            "Food Expenses",
            r#"[{"id":"cat-a"},{"id":"cat-b"}]"#,
            r#"[{"field":"amount","op":"lt","value":0}]"#
        ],
    )
    .unwrap();

    // Run remaining migrations including 1722717601000
    actual_storage::migrate(&mut conn).expect("Migrate should succeed");

    // Verify conditions now contains category filter
    let conditions_str: String = conn
        .query_row(
            "SELECT conditions FROM custom_reports WHERE id = 'rep-1'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let conditions: serde_json::Value = serde_json::from_str(&conditions_str).unwrap();
    assert_eq!(conditions.as_array().unwrap().len(), 2);

    let cat_cond = conditions
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c.get("field").and_then(|f| f.as_str()) == Some("category"))
        .expect("Category condition must be added");
    assert_eq!(
        cat_cond.get("value").unwrap(),
        &serde_json::json!(["cat-a", "cat-b"])
    );

    // Verify selected_categories is now NULL
    let sel_cats: Option<String> = conn
        .query_row(
            "SELECT selected_categories FROM custom_reports WHERE id = 'rep-1'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(sel_cats.is_none());
}
