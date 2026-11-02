// Upstream Actual Budget Migration Runner
// Replicates packages/loot-core/src/server/migrate/migrations.ts

use crate::error::StorageError;
use rusqlite::Connection;
use std::collections::HashSet;

pub const ADDITIVE_ONLY_CUTOFF: i64 = 1780606215001;
pub const INIT_SQL: &str = include_str!("../../../specs/sql/init.sql");

#[derive(Clone, Copy)]
pub enum MigrationHandler {
    Sql(&'static str),
    Custom(fn(&Connection) -> Result<(), StorageError>),
}

pub struct Migration {
    pub id: i64,
    pub name: &'static str,
    pub handler: MigrationHandler,
}

pub fn get_available_migrations() -> &'static [Migration] {
    &MIGRATIONS
}

static MIGRATIONS: [Migration; 60] = [
    Migration {
        id: 1548957970627,
        name: "1548957970627_remove-db-version.sql",
        handler: MigrationHandler::Sql(include_str!(
            "../migrations/1548957970627_remove-db-version.sql"
        )),
    },
    Migration {
        id: 1550601598648,
        name: "1550601598648_payees.sql",
        handler: MigrationHandler::Sql(include_str!("../migrations/1550601598648_payees.sql")),
    },
    Migration {
        id: 1555786194328,
        name: "1555786194328_remove_category_group_unique.sql",
        handler: MigrationHandler::Sql(include_str!(
            "../migrations/1555786194328_remove_category_group_unique.sql"
        )),
    },
    Migration {
        id: 1561751833510,
        name: "1561751833510_indexes.sql",
        handler: MigrationHandler::Sql(include_str!("../migrations/1561751833510_indexes.sql")),
    },
    Migration {
        id: 1567699552727,
        name: "1567699552727_budget.sql",
        handler: MigrationHandler::Sql(include_str!("../migrations/1567699552727_budget.sql")),
    },
    Migration {
        id: 1582384163573,
        name: "1582384163573_cleared.sql",
        handler: MigrationHandler::Sql(include_str!("../migrations/1582384163573_cleared.sql")),
    },
    Migration {
        id: 1597756566448,
        name: "1597756566448_rules.sql",
        handler: MigrationHandler::Sql(include_str!("../migrations/1597756566448_rules.sql")),
    },
    Migration {
        id: 1608652596043,
        name: "1608652596043_parent_field.sql",
        handler: MigrationHandler::Sql(include_str!(
            "../migrations/1608652596043_parent_field.sql"
        )),
    },
    Migration {
        id: 1608652596044,
        name: "1608652596044_trans_views.sql",
        handler: MigrationHandler::Sql(include_str!("../migrations/1608652596044_trans_views.sql")),
    },
    Migration {
        id: 1612625548236,
        name: "1612625548236_optimize.sql",
        handler: MigrationHandler::Sql(include_str!("../migrations/1612625548236_optimize.sql")),
    },
    Migration {
        id: 1614782639336,
        name: "1614782639336_trans_views2.sql",
        handler: MigrationHandler::Sql(include_str!(
            "../migrations/1614782639336_trans_views2.sql"
        )),
    },
    Migration {
        id: 1615745967948,
        name: "1615745967948_meta.sql",
        handler: MigrationHandler::Sql(include_str!("../migrations/1615745967948_meta.sql")),
    },
    Migration {
        id: 1616167010796,
        name: "1616167010796_accounts_order.sql",
        handler: MigrationHandler::Sql(include_str!(
            "../migrations/1616167010796_accounts_order.sql"
        )),
    },
    Migration {
        id: 1618975177358,
        name: "1618975177358_schedules.sql",
        handler: MigrationHandler::Sql(include_str!("../migrations/1618975177358_schedules.sql")),
    },
    Migration {
        id: 1632571489012,
        name: "1632571489012_remove_cache.js",
        handler: MigrationHandler::Custom(migration_1632571489012),
    },
    Migration {
        id: 1679728867040,
        name: "1679728867040_rules_conditions.sql",
        handler: MigrationHandler::Sql(include_str!(
            "../migrations/1679728867040_rules_conditions.sql"
        )),
    },
    Migration {
        id: 1681115033845,
        name: "1681115033845_add_schedule_name.sql",
        handler: MigrationHandler::Sql(include_str!(
            "../migrations/1681115033845_add_schedule_name.sql"
        )),
    },
    Migration {
        id: 1682974838138,
        name: "1682974838138_remove_payee_rules.sql",
        handler: MigrationHandler::Sql(include_str!(
            "../migrations/1682974838138_remove_payee_rules.sql"
        )),
    },
    Migration {
        id: 1685007876842,
        name: "1685007876842_add_category_hidden.sql",
        handler: MigrationHandler::Sql(include_str!(
            "../migrations/1685007876842_add_category_hidden.sql"
        )),
    },
    Migration {
        id: 1686139660866,
        name: "1686139660866_remove_account_type.sql",
        handler: MigrationHandler::Sql(include_str!(
            "../migrations/1686139660866_remove_account_type.sql"
        )),
    },
    Migration {
        id: 1688749527273,
        name: "1688749527273_transaction_filters.sql",
        handler: MigrationHandler::Sql(include_str!(
            "../migrations/1688749527273_transaction_filters.sql"
        )),
    },
    Migration {
        id: 1688841238000,
        name: "1688841238000_add_account_type.sql",
        handler: MigrationHandler::Sql(include_str!(
            "../migrations/1688841238000_add_account_type.sql"
        )),
    },
    Migration {
        id: 1691233396000,
        name: "1691233396000_add_schedule_next_date_tombstone.sql",
        handler: MigrationHandler::Sql(include_str!(
            "../migrations/1691233396000_add_schedule_next_date_tombstone.sql"
        )),
    },
    Migration {
        id: 1694438752000,
        name: "1694438752000_add_goal_targets.sql",
        handler: MigrationHandler::Sql(include_str!(
            "../migrations/1694438752000_add_goal_targets.sql"
        )),
    },
    Migration {
        id: 1697046240000,
        name: "1697046240000_add_reconciled.sql",
        handler: MigrationHandler::Sql(include_str!(
            "../migrations/1697046240000_add_reconciled.sql"
        )),
    },
    Migration {
        id: 1704572023730,
        name: "1704572023730_add_account_sync_source.sql",
        handler: MigrationHandler::Sql(include_str!(
            "../migrations/1704572023730_add_account_sync_source.sql"
        )),
    },
    Migration {
        id: 1704572023731,
        name: "1704572023731_add_missing_goCardless_sync_source.sql",
        handler: MigrationHandler::Sql(include_str!(
            "../migrations/1704572023731_add_missing_goCardless_sync_source.sql"
        )),
    },
    Migration {
        id: 1707267033000,
        name: "1707267033000_reports.sql",
        handler: MigrationHandler::Sql(include_str!("../migrations/1707267033000_reports.sql")),
    },
    Migration {
        id: 1712784523000,
        name: "1712784523000_unhide_input_group.sql",
        handler: MigrationHandler::Sql(include_str!(
            "../migrations/1712784523000_unhide_input_group.sql"
        )),
    },
    Migration {
        id: 1716359441000,
        name: "1716359441000_include_current.sql",
        handler: MigrationHandler::Sql(include_str!(
            "../migrations/1716359441000_include_current.sql"
        )),
    },
    Migration {
        id: 1720310586000,
        name: "1720310586000_link_transfer_schedules.sql",
        handler: MigrationHandler::Sql(include_str!(
            "../migrations/1720310586000_link_transfer_schedules.sql"
        )),
    },
    Migration {
        id: 1720664867241,
        name: "1720664867241_add_payee_favorite.sql",
        handler: MigrationHandler::Sql(include_str!(
            "../migrations/1720664867241_add_payee_favorite.sql"
        )),
    },
    Migration {
        id: 1720665000000,
        name: "1720665000000_goal_context.sql",
        handler: MigrationHandler::Sql(include_str!(
            "../migrations/1720665000000_goal_context.sql"
        )),
    },
    Migration {
        id: 1722717601000,
        name: "1722717601000_reports_move_selected_categories.js",
        handler: MigrationHandler::Custom(migration_1722717601000),
    },
    Migration {
        id: 1722804019000,
        name: "1722804019000_create_dashboard_table.js",
        handler: MigrationHandler::Custom(migration_1722804019000),
    },
    Migration {
        id: 1723665565000,
        name: "1723665565000_prefs.js",
        handler: MigrationHandler::Custom(migration_1723665565000),
    },
    Migration {
        id: 1730744182000,
        name: "1730744182000_fix_dashboard_table.sql",
        handler: MigrationHandler::Sql(include_str!(
            "../migrations/1730744182000_fix_dashboard_table.sql"
        )),
    },
    Migration {
        id: 1736640000000,
        name: "1736640000000_custom_report_sorting.sql",
        handler: MigrationHandler::Sql(include_str!(
            "../migrations/1736640000000_custom_report_sorting.sql"
        )),
    },
    Migration {
        id: 1737158400000,
        name: "1737158400000_add_learn_categories_to_payees.sql",
        handler: MigrationHandler::Sql(include_str!(
            "../migrations/1737158400000_add_learn_categories_to_payees.sql"
        )),
    },
    Migration {
        id: 1738491452000,
        name: "1738491452000_sorting_rename.sql",
        handler: MigrationHandler::Sql(include_str!(
            "../migrations/1738491452000_sorting_rename.sql"
        )),
    },
    Migration {
        id: 1739139550000,
        name: "1739139550000_bank_sync_page.sql",
        handler: MigrationHandler::Sql(include_str!(
            "../migrations/1739139550000_bank_sync_page.sql"
        )),
    },
    Migration {
        id: 1740506588539,
        name: "1740506588539_add_last_reconciled_at.sql",
        handler: MigrationHandler::Sql(include_str!(
            "../migrations/1740506588539_add_last_reconciled_at.sql"
        )),
    },
    Migration {
        id: 1745425408000,
        name: "1745425408000_update_budgetType_pref.sql",
        handler: MigrationHandler::Sql(include_str!(
            "../migrations/1745425408000_update_budgetType_pref.sql"
        )),
    },
    Migration {
        id: 1749799110000,
        name: "1749799110000_add_tags.sql",
        handler: MigrationHandler::Sql(include_str!("../migrations/1749799110000_add_tags.sql")),
    },
    Migration {
        id: 1749799110001,
        name: "1749799110001_tags_tombstone.sql",
        handler: MigrationHandler::Sql(include_str!(
            "../migrations/1749799110001_tags_tombstone.sql"
        )),
    },
    Migration {
        id: 1754611200000,
        name: "1754611200000_add_category_template_settings.sql",
        handler: MigrationHandler::Sql(include_str!(
            "../migrations/1754611200000_add_category_template_settings.sql"
        )),
    },
    Migration {
        id: 1759260219000,
        name: "1759260219000_add_trim_interval_report_setting.sql",
        handler: MigrationHandler::Sql(include_str!(
            "../migrations/1759260219000_add_trim_interval_report_setting.sql"
        )),
    },
    Migration {
        id: 1759842823172,
        name: "1759842823172_add_isGlobal_to_preferences.sql",
        handler: MigrationHandler::Sql(include_str!(
            "../migrations/1759842823172_add_isGlobal_to_preferences.sql"
        )),
    },
    Migration {
        id: 1762178745667,
        name: "1762178745667_rename_csv_skip_lines_pref.sql",
        handler: MigrationHandler::Sql(include_str!(
            "../migrations/1762178745667_rename_csv_skip_lines_pref.sql"
        )),
    },
    Migration {
        id: 1765518577215,
        name: "1765518577215_multiple_dashboards.js",
        handler: MigrationHandler::Custom(migration_1765518577215),
    },
    Migration {
        id: 1768872504000,
        name: "1768872504000_add_payee_locations.sql",
        handler: MigrationHandler::Sql(include_str!(
            "../migrations/1768872504000_add_payee_locations.sql"
        )),
    },
    Migration {
        id: 1769000000000,
        name: "1769000000000_add_custom_upcoming_length.sql",
        handler: MigrationHandler::Sql(include_str!(
            "../migrations/1769000000000_add_custom_upcoming_length.sql"
        )),
    },
    Migration {
        id: 1778510362740,
        name: "1778510362740_add_cleanup_groups_and_def.sql",
        handler: MigrationHandler::Sql(include_str!(
            "../migrations/1778510362740_add_cleanup_groups_and_def.sql"
        )),
    },
    Migration {
        id: 1780099200000,
        name: "1780099200000_add_show_trend_lines_report_setting.sql",
        handler: MigrationHandler::Sql(include_str!(
            "../migrations/1780099200000_add_show_trend_lines_report_setting.sql"
        )),
    },
    Migration {
        id: 1780327681000,
        name: "1780327681000_add_tags_hidden.sql",
        handler: MigrationHandler::Sql(include_str!(
            "../migrations/1780327681000_add_tags_hidden.sql"
        )),
    },
    Migration {
        id: 1780606215000,
        name: "1780606215000_add_bank_sync_status.sql",
        handler: MigrationHandler::Sql(include_str!(
            "../migrations/1780606215000_add_bank_sync_status.sql"
        )),
    },
    Migration {
        id: 1780606215001,
        name: "1780606215001_add_performance_indexes.sql",
        handler: MigrationHandler::Sql(include_str!(
            "../migrations/1780606215001_add_performance_indexes.sql"
        )),
    },
    Migration {
        id: 1783004650757,
        name: "1783004650757_schedule_sort_order.sql",
        handler: MigrationHandler::Sql(include_str!(
            "../migrations/1783004650757_schedule_sort_order.sql"
        )),
    },
    Migration {
        id: 1787013118115,
        name: "1787013118115_add_account_groups.sql",
        handler: MigrationHandler::Sql(include_str!(
            "../migrations/1787013118115_add_account_groups.sql"
        )),
    },
    Migration {
        id: 1788468782000,
        name: "1788468782000_add_messages_pending.js",
        handler: MigrationHandler::Custom(migration_1788468782000),
    },
];

pub fn get_applied_migrations(conn: &Connection) -> Result<Vec<i64>, StorageError> {
    let mut stmt = conn.prepare("SELECT id FROM __migrations__ ORDER BY id ASC")?;
    let rows = stmt.query_map([], |row| row.get(0))?;
    let mut applied = Vec::new();
    for r in rows {
        applied.push(r?);
    }
    Ok(applied)
}

pub fn patch_bad_migrations(conn: &Connection) -> Result<(), StorageError> {
    let bad_filters_migration: i64 = 1685375406832;
    let new_filters_migration: i64 = 1688749527273;
    let has_bad: bool = conn
        .query_row(
            "SELECT 1 FROM __migrations__ WHERE id = ?1",
            [bad_filters_migration],
            |_| Ok(true),
        )
        .unwrap_or(false);
    if has_bad {
        conn.execute(
            "DELETE FROM __migrations__ WHERE id = ?1",
            [bad_filters_migration],
        )?;
        conn.execute(
            "INSERT OR IGNORE INTO __migrations__ (id) VALUES (?1)",
            [new_filters_migration],
        )?;
    }
    Ok(())
}

pub fn check_database_validity(
    applied_ids: &[i64],
    available_ids: &[i64],
) -> Result<(), StorageError> {
    if available_ids.is_empty() && !applied_ids.is_empty() {
        return Err(StorageError::OutOfSyncMigrations(
            "No migrations found on disk for a migrated database".to_string(),
        ));
    }

    let available_set: HashSet<i64> = available_ids.iter().copied().collect();
    let unknown_ids: Vec<i64> = applied_ids
        .iter()
        .copied()
        .filter(|id| !available_set.contains(id))
        .collect();

    if unknown_ids.iter().any(|&id| id <= ADDITIVE_ONLY_CUTOFF) {
        return Err(StorageError::OutOfSyncMigrations(
            "Database contains unknown migration from before the additive-only era".to_string(),
        ));
    }

    let known_applied_ids: Vec<i64> = applied_ids
        .iter()
        .copied()
        .filter(|id| available_set.contains(id))
        .collect();

    if !unknown_ids.is_empty() && known_applied_ids.len() != available_ids.len() {
        return Err(StorageError::OutOfSyncMigrations(
            "Database touched by newer version is missing known migrations".to_string(),
        ));
    }

    let pre_cutoff_available: Vec<i64> = available_ids
        .iter()
        .copied()
        .filter(|&id| id <= ADDITIVE_ONLY_CUTOFF)
        .collect();
    let pre_cutoff_applied: Vec<i64> = known_applied_ids
        .iter()
        .copied()
        .filter(|&id| id <= ADDITIVE_ONLY_CUTOFF)
        .collect();

    if applied_ids.iter().any(|&id| id > ADDITIVE_ONLY_CUTOFF)
        && pre_cutoff_applied.len() != pre_cutoff_available.len()
    {
        return Err(StorageError::OutOfSyncMigrations(
            "Database has applied post-cutoff migrations but is missing pre-cutoff migrations"
                .to_string(),
        ));
    }

    for (applied, available) in pre_cutoff_applied.iter().zip(pre_cutoff_available.iter()) {
        if applied != available {
            return Err(StorageError::OutOfSyncMigrations(format!(
                "Migration mismatch: applied {} but expected {}",
                applied, available
            )));
        }
    }

    Ok(())
}

pub fn initialize_database(conn: &mut Connection) -> Result<(), StorageError> {
    let has_migrations_table: bool = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = '__migrations__'",
            [],
            |_| Ok(true),
        )
        .unwrap_or(false);

    if !has_migrations_table {
        conn.execute_batch(INIT_SQL)?;
    }

    Ok(())
}

pub fn migrate(conn: &mut Connection) -> Result<Vec<i64>, StorageError> {
    patch_bad_migrations(conn)?;
    let applied_ids = get_applied_migrations(conn)?;
    let available = get_available_migrations();
    let available_ids: Vec<i64> = available.iter().map(|m| m.id).collect();

    check_database_validity(&applied_ids, &available_ids)?;

    let mut newly_applied = Vec::new();
    for migration in available {
        if !applied_ids.contains(&migration.id) {
            match migration.handler {
                MigrationHandler::Sql(sql) => {
                    conn.execute_batch(sql)?;
                }
                MigrationHandler::Custom(func) => {
                    func(conn)?;
                }
            }
            conn.execute(
                "INSERT INTO __migrations__ (id) VALUES (?1)",
                [migration.id],
            )?;
            newly_applied.push(migration.id);
        }
    }

    Ok(newly_applied)
}

// Custom JS migrations translated to native Rust

fn migration_1632571489012(conn: &Connection) -> Result<(), StorageError> {
    conn.execute_batch(
        r#"
CREATE TABLE IF NOT EXISTS zero_budget_months
  (id TEXT PRIMARY KEY,
   buffered INTEGER DEFAULT 0);

CREATE TABLE IF NOT EXISTS zero_budgets
  (id TEXT PRIMARY KEY,
   month INTEGER,
   category TEXT,
   amount INTEGER DEFAULT 0,
   carryover INTEGER DEFAULT 0);

CREATE TABLE IF NOT EXISTS reflect_budgets
  (id TEXT PRIMARY KEY,
   month INTEGER,
   category TEXT,
   amount INTEGER DEFAULT 0,
   carryover INTEGER DEFAULT 0);

CREATE TABLE IF NOT EXISTS notes
  (id TEXT PRIMARY KEY,
   note TEXT);

CREATE TABLE IF NOT EXISTS kvcache (key TEXT PRIMARY KEY, value TEXT);
CREATE TABLE IF NOT EXISTS kvcache_key (id INTEGER PRIMARY KEY, key REAL);
"#,
    )?;

    let has_cells: bool = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'spreadsheet_cells'",
            [],
            |_| Ok(true),
        )
        .unwrap_or(false);

    if has_cells {
        let mut stmt = conn.prepare(
            "SELECT name, expr, cachedValue FROM spreadsheet_cells WHERE name LIKE 'budget%!budget-%'",
        )?;
        let rows = stmt.query_map([], |row| {
            let name: String = row.get(0)?;
            let expr: Option<String> = row.get(1)?;
            let cached_val: Option<String> = row.get(2)?;
            Ok((name, expr.or(cached_val).unwrap_or_default()))
        })?;

        let mut budget_items = Vec::new();
        for r in rows {
            budget_items.push(r?);
        }

        for (name, val) in budget_items {
            let parts: Vec<&str> = name.split('!').collect();
            if parts.len() != 2 {
                continue;
            }
            let sheet = parts[0];
            let rest = parts[1];
            let cat = match rest.strip_prefix("budget-") {
                Some(c) => c,
                None => continue,
            };

            let (is_report, num_str) = if let Some(stripped) = sheet.strip_prefix("budget-report") {
                (true, stripped)
            } else if let Some(stripped) = sheet.strip_prefix("budget") {
                (false, stripped)
            } else {
                continue;
            };

            if num_str.len() < 5 {
                continue;
            }
            let month_formatted = format!("{}-{}", &num_str[..4], &num_str[4..]);
            let dbmonth: i64 = num_str.parse().unwrap_or(0);
            let amount: i64 = val.parse().unwrap_or(0);

            let carryover_name = format!("{}!carryover-{}", sheet, cat);
            let carryover_val: Option<String> = conn
                .query_row(
                    "SELECT COALESCE(expr, cachedValue) FROM spreadsheet_cells WHERE name = ?1",
                    [&carryover_name],
                    |row| row.get(0),
                )
                .ok();
            let carryover = if carryover_val.as_deref() == Some("true") {
                1
            } else {
                0
            };

            let table = if is_report {
                "reflect_budgets"
            } else {
                "zero_budgets"
            };
            let id = format!("{}-{}", month_formatted, cat);
            let insert_sql = format!(
                "INSERT OR REPLACE INTO {} (id, month, category, amount, carryover) VALUES (?1, ?2, ?3, ?4, ?5)",
                table
            );
            conn.execute(
                &insert_sql,
                rusqlite::params![id, dbmonth, cat, amount, carryover],
            )?;
        }

        let mut stmt = conn.prepare(
            "SELECT name, expr, cachedValue FROM spreadsheet_cells WHERE name LIKE 'budget%!buffered'",
        )?;
        let rows = stmt.query_map([], |row| {
            let name: String = row.get(0)?;
            let expr: Option<String> = row.get(1)?;
            let cached_val: Option<String> = row.get(2)?;
            Ok((name, expr.or(cached_val).unwrap_or_default()))
        })?;

        let mut buffer_items = Vec::new();
        for r in rows {
            buffer_items.push(r?);
        }

        for (name, val) in buffer_items {
            if let Some(sheet) = name.strip_suffix("!buffered") {
                if let Some(num_str) = sheet.strip_prefix("budget") {
                    if num_str.len() >= 5 {
                        let month_formatted = format!("{}-{}", &num_str[..4], &num_str[4..]);
                        let amount: i64 = val.parse().unwrap_or(0);
                        conn.execute(
                            "INSERT OR REPLACE INTO zero_budget_months (id, buffered) VALUES (?1, ?2)",
                            rusqlite::params![month_formatted, amount],
                        )?;
                    }
                }
            }
        }

        let mut stmt = conn.prepare(
            "SELECT name, expr, cachedValue FROM spreadsheet_cells WHERE name LIKE 'notes!%'",
        )?;
        let rows = stmt.query_map([], |row| {
            let name: String = row.get(0)?;
            let expr: Option<String> = row.get(1)?;
            let cached_val: Option<String> = row.get(2)?;
            Ok((name, expr.or(cached_val).unwrap_or_default()))
        })?;

        let mut note_items = Vec::new();
        for r in rows {
            note_items.push(r?);
        }

        for (name, val) in note_items {
            if let Some(id) = name.strip_prefix("notes!") {
                let note_text = if let Ok(serde_json::Value::String(s)) =
                    serde_json::from_str::<serde_json::Value>(&val)
                {
                    s
                } else {
                    val
                };
                if !note_text.is_empty() {
                    conn.execute(
                        "INSERT OR REPLACE INTO notes (id, note) VALUES (?1, ?2)",
                        rusqlite::params![id, note_text],
                    )?;
                }
            }
        }

        conn.execute_batch(
            r#"
            DROP TABLE spreadsheet_cells;
            ANALYZE;
            "#,
        )?;
    }

    Ok(())
}

fn migration_1722717601000(conn: &Connection) -> Result<(), StorageError> {
    let mut stmt = conn.prepare("SELECT id FROM categories WHERE tombstone = 0")?;
    let active_cat_ids: HashSet<String> = stmt
        .query_map([], |row| row.get(0))?
        .filter_map(Result::ok)
        .collect();

    let mut stmt = conn.prepare(
        "SELECT id, selected_categories, conditions FROM custom_reports WHERE tombstone = 0 AND selected_categories IS NOT NULL",
    )?;
    let reports = stmt.query_map([], |row| {
        let id: String = row.get(0)?;
        let selected_categories: Option<String> = row.get(1)?;
        let conditions: Option<String> = row.get(2)?;
        Ok((id, selected_categories, conditions))
    })?;

    let mut report_updates = Vec::new();
    for r in reports {
        report_updates.push(r?);
    }

    for (id, selected_cats_str, conds_str) in report_updates {
        let selected_cats: Vec<serde_json::Value> = selected_cats_str
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();

        let selected_ids: Vec<String> = selected_cats
            .iter()
            .filter_map(|v| {
                v.get("id")
                    .and_then(|id_val| id_val.as_str())
                    .map(|s| s.to_string())
            })
            .collect();

        let are_all_selected = active_cat_ids
            .iter()
            .all(|cat_id| selected_ids.contains(cat_id));

        if are_all_selected {
            continue;
        }

        let mut conditions: Vec<serde_json::Value> = conds_str
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();

        let has_category_filter = conditions
            .iter()
            .any(|c| c.get("field").and_then(|f| f.as_str()) == Some("category"));

        if has_category_filter {
            continue;
        }

        let new_cond = serde_json::json!({
            "field": "category",
            "op": "oneOf",
            "value": selected_ids,
            "type": "category",
        });
        conditions.push(new_cond);

        let new_conds_str = serde_json::to_string(&conditions)?;
        conn.execute(
            "UPDATE custom_reports SET conditions = ?1 WHERE id = ?2",
            rusqlite::params![new_conds_str, id],
        )?;
    }

    conn.execute(
        "UPDATE custom_reports SET selected_categories = NULL WHERE tombstone = 0",
        [],
    )?;

    Ok(())
}

fn migration_1722804019000(conn: &Connection) -> Result<(), StorageError> {
    conn.execute_batch(
        r#"
        CREATE TABLE dashboard
          (id TEXT PRIMARY KEY,
           type TEXT,
           width INTEGER,
           height INTEGER,
           x INTEGER,
           y INTEGER,
           meta TEXT,
           tombstone INTEGER DEFAULT 0);
        "#,
    )?;

    let mut stmt = conn.prepare(
        "SELECT id FROM custom_reports WHERE tombstone = 0 ORDER BY name COLLATE NOCASE ASC",
    )?;
    let report_ids: Vec<String> = stmt
        .query_map([], |row| row.get(0))?
        .filter_map(Result::ok)
        .collect();

    if report_ids.is_empty() {
        type WidgetSpec = (&'static str, i64, i64, i64, i64, Option<String>);
        let default_widgets: [WidgetSpec; 12] = [
            (
                "summary-card", 3, 2, 0, 0,
                Some(serde_json::json!({
                    "name": "Total Income (YTD)",
                    "content": serde_json::json!({ "type": "sum", "fontSize": 20 }).to_string(),
                    "timeFrame": { "start": "2024-01-01", "end": "2024-12-31", "mode": "yearToDate" },
                    "conditions": [
                        { "field": "amount", "op": "gt", "value": 0 },
                        { "field": "account", "op": "onBudget", "value": "" },
                        { "field": "transfer", "op": "is", "value": false }
                    ],
                    "conditionsOp": "and"
                }).to_string())
            ),
            (
                "summary-card", 3, 2, 3, 0,
                Some(serde_json::json!({
                    "name": "Total Expenses (YTD)",
                    "content": serde_json::json!({ "type": "sum", "fontSize": 20 }).to_string(),
                    "timeFrame": { "start": "2024-01-01", "end": "2024-12-31", "mode": "yearToDate" },
                    "conditions": [
                        { "field": "amount", "op": "lt", "value": 0 },
                        { "field": "account", "op": "onBudget", "value": "" },
                        { "field": "transfer", "op": "is", "value": false }
                    ],
                    "conditionsOp": "and"
                }).to_string())
            ),
            (
                "summary-card", 3, 2, 6, 0,
                Some(serde_json::json!({
                    "name": "Avg Per Month",
                    "content": serde_json::json!({ "type": "avgPerMonth", "fontSize": 20 }).to_string(),
                    "timeFrame": { "start": "2024-01-01", "end": "2024-12-31", "mode": "yearToDate" },
                    "conditions": [
                        { "field": "amount", "op": "lt", "value": 0 },
                        { "field": "account", "op": "onBudget", "value": "" },
                        { "field": "transfer", "op": "is", "value": false }
                    ],
                    "conditionsOp": "and"
                }).to_string())
            ),
            (
                "summary-card", 3, 2, 9, 0,
                Some(serde_json::json!({
                    "name": "Avg Per Transaction",
                    "content": serde_json::json!({ "type": "avgPerTransact", "fontSize": 20 }).to_string(),
                    "timeFrame": { "start": "2024-01-01", "end": "2024-12-31", "mode": "yearToDate" },
                    "conditions": [
                        { "field": "amount", "op": "lt", "value": 0 },
                        { "field": "account", "op": "onBudget", "value": "" },
                        { "field": "transfer", "op": "is", "value": false }
                    ],
                    "conditionsOp": "and"
                }).to_string())
            ),
            (
                "net-worth-card", 6, 2, 0, 2,
                None
            ),
            (
                "cash-flow-card", 6, 2, 6, 2,
                None
            ),
            (
                "spending-card", 4, 2, 0, 5,
                Some(serde_json::json!({ "name": "This Month", "mode": "single-month" }).to_string())
            ),
            (
                "spending-card", 4, 2, 4, 5,
                Some(serde_json::json!({ "name": "Budget Overview", "mode": "budget" }).to_string())
            ),
            (
                "spending-card", 4, 2, 8, 5,
                Some(serde_json::json!({ "name": "3-Month Average", "mode": "average" }).to_string())
            ),
            (
                "calendar-card", 8, 4, 0, 8,
                Some(serde_json::json!({
                    "name": "Transaction Calendar",
                    "timeFrame": { "start": "2024-01-01", "end": "2024-03-31", "mode": "sliding-window" },
                    "conditions": [
                        { "field": "transfer", "op": "is", "value": false }
                    ],
                    "conditionsOp": "and"
                }).to_string())
            ),
            (
                "summary-card", 4, 2, 8, 8,
                Some(serde_json::json!({
                    "name": "Recent Net Worth Change",
                    "content": serde_json::json!({ "type": "sum", "fontSize": 32 }).to_string(),
                    "timeFrame": { "start": "2024-01-01", "end": "2024-03-31", "mode": "sliding-window" },
                    "conditions": [],
                    "conditionsOp": "and"
                }).to_string())
            ),
            (
                "markdown-card", 4, 2, 8, 10,
                Some(serde_json::json!({
                    "content": "## Dashboard Tips\n\nYou can add new widgets or edit existing widgets by using the buttons at the top of the page. Choose a widget type and customize it to fit your needs.\n\n**Moving cards:** Drag any card by its header to reposition it.\n\n**Deleting cards:** Click the three-dot menu on any card and select 'Remove'."
                }).to_string())
            ),
        ];

        for (widget_type, width, height, x, y, meta) in default_widgets {
            let id = uuid::Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO dashboard (id, type, width, height, x, y, meta) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![id, widget_type, width, height, x, y, meta],
            )?;
        }
    } else {
        conn.execute(
            "INSERT INTO dashboard (id, type, width, height, x, y) VALUES (?1, 'net-worth-card', 8, 2, 0, 0)",
            [uuid::Uuid::new_v4().to_string()],
        )?;
        conn.execute(
            "INSERT INTO dashboard (id, type, width, height, x, y) VALUES (?1, 'cash-flow-card', 4, 2, 8, 0)",
            [uuid::Uuid::new_v4().to_string()],
        )?;
        conn.execute(
            "INSERT INTO dashboard (id, type, width, height, x, y) VALUES (?1, 'spending-card', 4, 2, 0, 2)",
            [uuid::Uuid::new_v4().to_string()],
        )?;

        for (idx, report_id) in report_ids.into_iter().enumerate() {
            let id = uuid::Uuid::new_v4().to_string();
            let x = ((idx as i64) * 4) % 12;
            let y = 2 + ((idx as i64) / 3) * 2;
            let meta = serde_json::json!({ "id": report_id }).to_string();
            conn.execute(
                "INSERT INTO dashboard (id, type, width, height, x, y, meta) VALUES (?1, 'custom-report', 4, 2, ?2, ?3, ?4)",
                rusqlite::params![id, x, y, meta],
            )?;
        }
    }

    Ok(())
}

fn migration_1723665565000(conn: &Connection) -> Result<(), StorageError> {
    conn.execute_batch(
        r#"
        CREATE TABLE preferences
           (id TEXT PRIMARY KEY,
            value TEXT);
        "#,
    )?;
    Ok(())
}

fn migration_1765518577215(conn: &Connection) -> Result<(), StorageError> {
    conn.execute_batch(
        r#"
        CREATE TABLE dashboard_pages
          (id TEXT PRIMARY KEY,
           name TEXT,
           tombstone INTEGER DEFAULT 0);

        ALTER TABLE dashboard ADD COLUMN dashboard_page_id TEXT;
        "#,
    )?;

    let default_id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO dashboard_pages (id, name) VALUES (?1, 'Main')",
        [&default_id],
    )?;
    conn.execute("UPDATE dashboard SET dashboard_page_id = ?1", [&default_id])?;

    Ok(())
}

fn migration_1788468782000(conn: &Connection) -> Result<(), StorageError> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS messages_pending
          (dataset TEXT NOT NULL,
           row TEXT NOT NULL,
           column TEXT NOT NULL,
           timestamp TEXT NOT NULL,
           value TEXT NOT NULL,
           PRIMARY KEY (dataset, row, column));
        "#,
    )?;
    Ok(())
}
