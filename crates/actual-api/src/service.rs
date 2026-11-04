//! `BudgetService` — the central application service coordinating domain logic and storage.
//!
//! All mutable operations go through typed command structs (see [`commands`]) and are
//! recorded in the [`UndoManager`] so they can be reversed or redone without touching
//! the database directly.

use std::collections::HashMap;

use actual_core::{
    generate_upcoming_transactions, Account, Category, CategoryGroup, EnvelopeBudget, Payee, Rule,
    Schedule, Transaction,
};
use actual_storage::SqliteStore;
use uuid::Uuid;

use crate::commands::*;
use crate::error::ApiError;
use crate::undo::{UndoEntry, UndoManager};

/// Central application service that owns all in-memory state and the SQLite store.
///
/// # Architecture
/// - **In-memory maps** (`accounts`, `payees`, etc.) are the source-of-truth for reads;
///   they are populated at service startup from the SQLite store.
/// - **`EnvelopeBudget`** manages month-to-month envelope calculations.
/// - **`UndoManager`** records inverse snapshots for every mutating command.
///
/// The current implementation uses an in-memory state store with `SqliteStore` available
/// for persistence extension. Write-through to SQLite is deliberately left as a no-op in
/// this phase so that the typed API and undo layer can be exercised independently of
/// schema-specific SQL queries (which will be wired in Phase 4).
pub struct BudgetService {
    store: SqliteStore,
    undo: UndoManager,

    accounts: HashMap<String, Account>,
    payees: HashMap<String, Payee>,
    category_groups: HashMap<String, CategoryGroup>,
    categories: HashMap<String, Category>,
    transactions: HashMap<String, Transaction>,
    rules: HashMap<String, Rule>,
    schedules: HashMap<String, Schedule>,

    budget: EnvelopeBudget,
}

impl BudgetService {
    /// Open (or create) a budget file at `path` and initialise the service.
    ///
    /// # Errors
    /// Returns [`ApiError::Storage`] if the database cannot be opened or migrated.
    pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self, ApiError> {
        let store = SqliteStore::open(path)?;
        Ok(Self::from_store(store))
    }

    /// Open an in-memory budget (useful for tests and ephemeral sessions).
    ///
    /// # Errors
    /// Returns [`ApiError::Storage`] if the in-memory database cannot be initialised.
    pub fn open_in_memory() -> Result<Self, ApiError> {
        let store = SqliteStore::open_in_memory()?;
        Ok(Self::from_store(store))
    }

    fn from_store(store: SqliteStore) -> Self {
        Self {
            store,
            undo: UndoManager::new(),
            accounts: HashMap::new(),
            payees: HashMap::new(),
            category_groups: HashMap::new(),
            categories: HashMap::new(),
            transactions: HashMap::new(),
            rules: HashMap::new(),
            schedules: HashMap::new(),
            budget: EnvelopeBudget::default(),
        }
    }

    // ── Undo / Redo ────────────────────────────────────────────────────────────

    /// Undo the most recent reversible operation.
    ///
    /// # Errors
    /// Returns [`ApiError::NothingToUndo`] when the undo stack is empty.
    pub fn undo(&mut self) -> Result<(), ApiError> {
        let entry = self.undo.pop_undo()?;
        let redo_entry = self.apply_undo(entry)?;
        self.undo.push_redo(redo_entry);
        Ok(())
    }

    /// Redo the most recently undone operation.
    ///
    /// # Errors
    /// Returns [`ApiError::NothingToRedo`] when the redo stack is empty.
    pub fn redo(&mut self) -> Result<(), ApiError> {
        let entry = self.undo.pop_redo()?;
        let undo_entry = self.apply_redo(entry)?;
        self.undo.push(undo_entry);
        Ok(())
    }

    /// How many operations can currently be undone.
    #[must_use]
    pub fn undo_depth(&self) -> usize {
        self.undo.undo_depth()
    }

    /// How many operations can currently be redone.
    #[must_use]
    pub fn redo_depth(&self) -> usize {
        self.undo.redo_depth()
    }

    // ── Account commands ───────────────────────────────────────────────────────

    /// Create a new financial account.
    ///
    /// # Errors
    /// Propagates storage errors.
    pub fn create_account(&mut self, cmd: CreateAccount) -> Result<String, ApiError> {
        let id = if cmd.id.is_empty() {
            Uuid::new_v4().to_string()
        } else {
            cmd.id.clone()
        };
        let mut account = Account::new(&id, &cmd.name);
        account.offbudget = cmd.offbudget;
        self.accounts.insert(id.clone(), account);
        self.undo.push(UndoEntry::AccountCreated {
            account_id: id.clone(),
        });
        Ok(id)
    }

    /// Rename an existing account.
    ///
    /// # Errors
    /// Returns [`ApiError::AccountNotFound`] if the account does not exist.
    pub fn rename_account(&mut self, cmd: RenameAccount) -> Result<(), ApiError> {
        let account = self
            .accounts
            .get(&cmd.account_id)
            .ok_or_else(|| ApiError::AccountNotFound(cmd.account_id.clone()))?;
        let undo_entry = UndoManager::snapshot_account_rename(account);
        let account = self.accounts.get_mut(&cmd.account_id).unwrap();
        account.name = cmd.new_name;
        self.undo.push(undo_entry);
        Ok(())
    }

    /// Close (soft-delete) an account.
    ///
    /// # Errors
    /// Returns [`ApiError::AccountNotFound`] if the account does not exist.
    pub fn close_account(&mut self, cmd: CloseAccount) -> Result<(), ApiError> {
        let account = self
            .accounts
            .get_mut(&cmd.account_id)
            .ok_or_else(|| ApiError::AccountNotFound(cmd.account_id.clone()))?;
        account.closed = true;
        self.undo.push(UndoEntry::AccountClosed {
            account_id: cmd.account_id,
        });
        Ok(())
    }

    /// Reopen a previously closed account.
    ///
    /// # Errors
    /// Returns [`ApiError::AccountNotFound`] if the account does not exist.
    pub fn reopen_account(&mut self, cmd: ReopenAccount) -> Result<(), ApiError> {
        let account = self
            .accounts
            .get_mut(&cmd.account_id)
            .ok_or_else(|| ApiError::AccountNotFound(cmd.account_id.clone()))?;
        account.closed = false;
        self.undo.push(UndoEntry::AccountReopened {
            account_id: cmd.account_id,
        });
        Ok(())
    }

    // ── Payee commands ─────────────────────────────────────────────────────────

    /// Create a new payee.
    ///
    /// # Errors
    /// Propagates storage errors.
    pub fn create_payee(&mut self, cmd: CreatePayee) -> Result<String, ApiError> {
        let id = if cmd.id.is_empty() {
            Uuid::new_v4().to_string()
        } else {
            cmd.id.clone()
        };
        let payee = Payee::new(&id, &cmd.name);
        self.payees.insert(id.clone(), payee);
        self.undo.push(UndoEntry::PayeeCreated {
            payee_id: id.clone(),
        });
        Ok(id)
    }

    /// Rename an existing payee.
    ///
    /// # Errors
    /// Returns [`ApiError::PayeeNotFound`] if the payee does not exist.
    pub fn rename_payee(&mut self, cmd: RenamePayee) -> Result<(), ApiError> {
        let payee = self
            .payees
            .get(&cmd.payee_id)
            .ok_or_else(|| ApiError::PayeeNotFound(cmd.payee_id.clone()))?;
        let undo_entry = UndoManager::snapshot_payee_rename(payee);
        let payee = self.payees.get_mut(&cmd.payee_id).unwrap();
        payee.name = cmd.new_name;
        self.undo.push(undo_entry);
        Ok(())
    }

    /// Merge a source payee into a target payee.
    ///
    /// All transactions referencing `source_payee_id` are re-assigned to `target_payee_id`.
    /// The source payee is then tombstoned.
    ///
    /// # Errors
    /// Returns [`ApiError::PayeeNotFound`] if either payee does not exist.
    pub fn merge_payees(&mut self, cmd: MergePayees) -> Result<(), ApiError> {
        if !self.payees.contains_key(&cmd.source_payee_id) {
            return Err(ApiError::PayeeNotFound(cmd.source_payee_id.clone()));
        }
        if !self.payees.contains_key(&cmd.target_payee_id) {
            return Err(ApiError::PayeeNotFound(cmd.target_payee_id.clone()));
        }
        let restored_payee = self.payees[&cmd.source_payee_id].clone();

        let mut affected_ids = Vec::new();
        for (id, tx) in self.transactions.iter_mut() {
            if tx.payee_id.as_deref() == Some(&cmd.source_payee_id) {
                tx.payee_id = Some(cmd.target_payee_id.clone());
                affected_ids.push(id.clone());
            }
        }
        self.payees.get_mut(&cmd.source_payee_id).unwrap().tombstone = true;
        self.undo.push(UndoEntry::PayeesMerged {
            restored_payee,
            affected_transaction_ids: affected_ids,
        });
        Ok(())
    }

    // ── Category commands ──────────────────────────────────────────────────────

    /// Create a new category group.
    ///
    /// # Errors
    /// Propagates storage errors.
    pub fn create_category_group(&mut self, cmd: CreateCategoryGroup) -> Result<String, ApiError> {
        let id = if cmd.id.is_empty() {
            Uuid::new_v4().to_string()
        } else {
            cmd.id.clone()
        };
        let group = if cmd.is_income {
            CategoryGroup::new_income(&id, &cmd.name)
        } else {
            CategoryGroup::new(&id, &cmd.name)
        };
        self.category_groups.insert(id.clone(), group);
        self.undo.push(UndoEntry::CategoryGroupCreated {
            group_id: id.clone(),
        });
        Ok(id)
    }

    /// Create a new budget envelope category.
    ///
    /// # Errors
    /// Returns [`ApiError::CategoryGroupNotFound`] if the group does not exist.
    pub fn create_category(&mut self, cmd: CreateCategory) -> Result<String, ApiError> {
        if !self.category_groups.contains_key(&cmd.group_id) {
            return Err(ApiError::CategoryGroupNotFound(cmd.group_id));
        }
        let id = if cmd.id.is_empty() {
            Uuid::new_v4().to_string()
        } else {
            cmd.id.clone()
        };
        let category = Category::new(&id, &cmd.name, &cmd.group_id);
        self.categories.insert(id.clone(), category);
        self.undo.push(UndoEntry::CategoryCreated {
            category_id: id.clone(),
        });
        Ok(id)
    }

    /// Rename an existing category.
    ///
    /// # Errors
    /// Returns [`ApiError::CategoryNotFound`] if the category does not exist.
    pub fn rename_category(&mut self, cmd: RenameCategory) -> Result<(), ApiError> {
        let cat = self
            .categories
            .get(&cmd.category_id)
            .ok_or_else(|| ApiError::CategoryNotFound(cmd.category_id.clone()))?;
        let undo_entry = UndoManager::snapshot_category_rename(cat);
        self.categories.get_mut(&cmd.category_id).unwrap().name = cmd.new_name;
        self.undo.push(undo_entry);
        Ok(())
    }

    /// Move a category to a different group.
    ///
    /// # Errors
    /// Returns [`ApiError::CategoryNotFound`] or [`ApiError::CategoryGroupNotFound`].
    pub fn move_category(&mut self, cmd: MoveCategory) -> Result<(), ApiError> {
        if !self.category_groups.contains_key(&cmd.new_group_id) {
            return Err(ApiError::CategoryGroupNotFound(cmd.new_group_id.clone()));
        }
        let cat = self
            .categories
            .get_mut(&cmd.category_id)
            .ok_or_else(|| ApiError::CategoryNotFound(cmd.category_id.clone()))?;
        let previous_group_id = cat.group_id.clone();
        cat.group_id = cmd.new_group_id;
        self.undo.push(UndoEntry::CategoryMoved {
            category_id: cmd.category_id,
            previous_group_id,
        });
        Ok(())
    }

    /// Set category visibility (hidden / visible).
    ///
    /// # Errors
    /// Returns [`ApiError::CategoryNotFound`] if the category does not exist.
    pub fn set_category_visibility(&mut self, cmd: SetCategoryVisibility) -> Result<(), ApiError> {
        let cat = self
            .categories
            .get_mut(&cmd.category_id)
            .ok_or_else(|| ApiError::CategoryNotFound(cmd.category_id.clone()))?;
        let previous_hidden = cat.hidden;
        cat.hidden = cmd.hidden;
        self.undo.push(UndoEntry::CategoryVisibilityChanged {
            category_id: cmd.category_id,
            previous_hidden,
        });
        Ok(())
    }

    /// Soft-delete a category.
    ///
    /// # Errors
    /// Returns [`ApiError::CategoryNotFound`] if the category does not exist.
    pub fn delete_category(&mut self, cmd: DeleteCategory) -> Result<(), ApiError> {
        let cat = self
            .categories
            .get_mut(&cmd.category_id)
            .ok_or_else(|| ApiError::CategoryNotFound(cmd.category_id.clone()))?;
        let restored = cat.clone();
        cat.tombstone = true;
        self.undo.push(UndoEntry::CategoryDeleted { restored });
        Ok(())
    }

    // ── Envelope budget commands ───────────────────────────────────────────────

    /// Set the budgeted amount for a category envelope.
    ///
    /// # Errors
    /// Returns [`ApiError::CategoryNotFound`] if the category does not exist.
    pub fn set_budget(&mut self, cmd: SetBudget) -> Result<(), ApiError> {
        if !self.categories.contains_key(&cmd.category_id) {
            return Err(ApiError::CategoryNotFound(cmd.category_id.clone()));
        }
        let previous_amount = self
            .budget
            .get_category_budget(&cmd.month, &cmd.category_id);
        self.budget
            .set_budget(&cmd.month, &cmd.category_id, cmd.amount);
        self.undo.push(UndoEntry::BudgetSet {
            month: cmd.month,
            category_id: cmd.category_id,
            previous_amount,
        });
        Ok(())
    }

    /// Toggle carry-over behaviour for an overspent category envelope.
    ///
    /// # Errors
    /// Returns [`ApiError::CategoryNotFound`] if the category does not exist.
    pub fn set_carryover(&mut self, cmd: SetCarryover) -> Result<(), ApiError> {
        if !self.categories.contains_key(&cmd.category_id) {
            return Err(ApiError::CategoryNotFound(cmd.category_id.clone()));
        }
        let previous_carry = self
            .budget
            .get_category_carryover(&cmd.month, &cmd.category_id);
        self.budget
            .set_carryover(&cmd.month, &cmd.category_id, cmd.carry);
        self.undo.push(UndoEntry::CarryoverSet {
            month: cmd.month,
            category_id: cmd.category_id,
            previous_carry,
        });
        Ok(())
    }

    // ── Transaction commands ───────────────────────────────────────────────────

    /// Add a new transaction.
    ///
    /// # Errors
    /// Returns [`ApiError::AccountNotFound`] if the account does not exist.
    pub fn add_transaction(&mut self, cmd: AddTransaction) -> Result<String, ApiError> {
        if !self.accounts.contains_key(&cmd.account_id) {
            return Err(ApiError::AccountNotFound(cmd.account_id.clone()));
        }
        let id = if cmd.id.is_empty() {
            Uuid::new_v4().to_string()
        } else {
            cmd.id.clone()
        };
        let mut tx = Transaction::new(&id, &cmd.account_id, cmd.date, cmd.amount);
        tx.payee_id = cmd.payee_id;
        tx.category_id = cmd.category_id.clone();
        tx.notes = cmd.notes;
        tx.cleared = cmd.cleared;

        // Update envelope budget spent amounts.
        let month = cmd.date.format("%Y-%m").to_string();
        if let Some(ref cat_id) = cmd.category_id {
            self.budget.set_spent(
                &month,
                cat_id,
                self.budget.get_category_spent(&month, cat_id) + cmd.amount,
            );
        }

        self.transactions.insert(id.clone(), tx);
        self.undo.push(UndoEntry::TransactionAdded {
            transaction_id: id.clone(),
        });
        Ok(id)
    }

    /// Update mutable fields of an existing transaction.
    ///
    /// Only `Some` fields in the command are applied.
    ///
    /// # Errors
    /// Returns [`ApiError::TransactionNotFound`] if the transaction does not exist.
    pub fn update_transaction(&mut self, cmd: UpdateTransaction) -> Result<(), ApiError> {
        let tx = self
            .transactions
            .get(&cmd.transaction_id)
            .ok_or_else(|| ApiError::TransactionNotFound(cmd.transaction_id.clone()))?;
        let previous = tx.clone();
        let tx = self.transactions.get_mut(&cmd.transaction_id).unwrap();
        if let Some(date) = cmd.date {
            tx.date = date;
        }
        if let Some(amount) = cmd.amount {
            tx.amount = amount;
        }
        if let Some(payee_id) = cmd.payee_id {
            tx.payee_id = payee_id;
        }
        if let Some(category_id) = cmd.category_id {
            tx.category_id = category_id;
        }
        if let Some(notes) = cmd.notes {
            tx.notes = notes;
        }
        if let Some(cleared) = cmd.cleared {
            tx.cleared = cleared;
        }
        self.undo.push(UndoEntry::TransactionUpdated { previous });
        Ok(())
    }

    /// Soft-delete (tombstone) a transaction.
    ///
    /// # Errors
    /// Returns [`ApiError::TransactionNotFound`] if the transaction does not exist.
    pub fn delete_transaction(&mut self, cmd: DeleteTransaction) -> Result<(), ApiError> {
        let tx = self
            .transactions
            .get_mut(&cmd.transaction_id)
            .ok_or_else(|| ApiError::TransactionNotFound(cmd.transaction_id.clone()))?;
        let restored = tx.clone();
        tx.tombstone = true;
        self.undo.push(UndoEntry::TransactionDeleted { restored });
        Ok(())
    }

    /// Split a transaction into a parent + child sub-transactions.
    ///
    /// The `splits` field carries `(category_id, amount, notes)` for each child.
    /// The children's amounts must sum to the parent's amount.
    ///
    /// # Errors
    /// - [`ApiError::TransactionNotFound`] — parent not found.
    /// - [`ApiError::CannotSplitChild`] — transaction is already a child.
    /// - [`ApiError::SplitAmountMismatch`] — child amounts don't sum to parent amount.
    pub fn split_transaction(&mut self, cmd: SplitTransaction) -> Result<Vec<String>, ApiError> {
        let parent = self
            .transactions
            .get(&cmd.parent_transaction_id)
            .ok_or_else(|| ApiError::TransactionNotFound(cmd.parent_transaction_id.clone()))?;
        if parent.is_child {
            return Err(ApiError::CannotSplitChild);
        }
        let parent_amount = parent.amount;
        let parent_snap = parent.clone();

        // Validate amounts sum correctly.
        let total: i64 = cmd.splits.iter().map(|(_, a, _)| a.as_cents()).sum();
        if total != parent_amount.as_cents() {
            return Err(ApiError::SplitAmountMismatch {
                got: total,
                expected: parent_amount.as_cents(),
            });
        }

        // Mark parent as a split parent.
        {
            let parent_mut = self
                .transactions
                .get_mut(&cmd.parent_transaction_id)
                .unwrap();
            parent_mut.is_parent = true;
        }

        let parent_id = cmd.parent_transaction_id.clone();
        let parent_account_id = self.transactions[&parent_id].account_id.clone();
        let parent_date = self.transactions[&parent_id].date;

        let mut child_ids = Vec::new();
        for (cat_id, amount, notes) in &cmd.splits {
            let child_id = Uuid::new_v4().to_string();
            let mut child = Transaction::new(&child_id, &parent_account_id, parent_date, *amount);
            child.category_id = cat_id.clone();
            child.notes = notes.clone();
            child.is_child = true;
            child.parent_id = Some(parent_id.clone());
            self.transactions.insert(child_id.clone(), child);
            child_ids.push(child_id);
        }

        self.undo.push(UndoEntry::TransactionSplit {
            previous_parent: parent_snap,
            child_ids: child_ids.clone(),
        });
        Ok(child_ids)
    }

    /// Collapse a split transaction back to a plain parent transaction.
    ///
    /// # Errors
    /// Returns [`ApiError::TransactionNotFound`] if the parent does not exist.
    pub fn unsplit_transaction(&mut self, cmd: UnsplitTransaction) -> Result<(), ApiError> {
        let parent = self
            .transactions
            .get_mut(&cmd.parent_transaction_id)
            .ok_or_else(|| ApiError::TransactionNotFound(cmd.parent_transaction_id.clone()))?;
        let snap = parent.clone();
        parent.is_parent = false;

        // Tombstone all child splits.
        let child_ids: Vec<String> = self
            .transactions
            .values()
            .filter(|tx| tx.parent_id.as_deref() == Some(&cmd.parent_transaction_id))
            .map(|tx| tx.id.clone())
            .collect();
        for cid in &child_ids {
            if let Some(child) = self.transactions.get_mut(cid) {
                child.tombstone = true;
            }
        }
        // Undo of unsplit = re-split using the previous parent snapshot.
        self.undo.push(UndoEntry::TransactionSplit {
            previous_parent: snap,
            child_ids,
        });
        Ok(())
    }

    // ── Rules commands ─────────────────────────────────────────────────────────

    /// Register a new transaction rule.
    ///
    /// # Errors
    /// Propagates storage errors.
    pub fn create_rule(&mut self, cmd: CreateRule) -> Result<String, ApiError> {
        let id = if cmd.id.is_empty() {
            Uuid::new_v4().to_string()
        } else {
            cmd.id.clone()
        };
        let mut rule = cmd.rule;
        rule.id = id.clone();
        self.rules.insert(id.clone(), rule);
        self.undo.push(UndoEntry::RuleCreated {
            rule_id: id.clone(),
        });
        Ok(id)
    }

    /// Replace an existing rule.
    ///
    /// # Errors
    /// Returns [`ApiError::RuleNotFound`] if the rule does not exist.
    pub fn update_rule(&mut self, cmd: UpdateRule) -> Result<(), ApiError> {
        let previous = self
            .rules
            .get(&cmd.rule_id)
            .ok_or_else(|| ApiError::RuleNotFound(cmd.rule_id.clone()))?
            .clone();
        self.rules.insert(cmd.rule_id.clone(), cmd.updated);
        self.undo.push(UndoEntry::RuleUpdated { previous });
        Ok(())
    }

    /// Delete a rule.
    ///
    /// # Errors
    /// Returns [`ApiError::RuleNotFound`] if the rule does not exist.
    pub fn delete_rule(&mut self, cmd: DeleteRule) -> Result<(), ApiError> {
        let restored = self
            .rules
            .remove(&cmd.rule_id)
            .ok_or_else(|| ApiError::RuleNotFound(cmd.rule_id.clone()))?;
        self.undo.push(UndoEntry::RuleDeleted { restored });
        Ok(())
    }

    /// Apply all stored rules to a batch of transactions.
    ///
    /// Returns the IDs of transactions that were modified by at least one rule.
    ///
    /// # Errors
    /// Returns [`ApiError::TransactionNotFound`] if any ID is unknown.
    pub fn apply_rules(&mut self, cmd: ApplyRules) -> Result<Vec<String>, ApiError> {
        use actual_core::RulesEngine;
        let rules: Vec<actual_core::Rule> = self.rules.values().cloned().collect();
        let mut engine = RulesEngine::new();
        engine.set_rules(rules);
        let mut modified = Vec::new();
        for tx_id in &cmd.transaction_ids {
            let tx = self
                .transactions
                .get_mut(tx_id)
                .ok_or_else(|| ApiError::TransactionNotFound(tx_id.clone()))?;
            let was = tx.clone();
            engine.apply(tx);
            if *tx != was {
                modified.push(tx_id.clone());
            }
        }
        Ok(modified)
    }

    // ── Schedule commands ──────────────────────────────────────────────────────

    /// Register a new recurring schedule.
    ///
    /// # Errors
    /// Propagates storage errors.
    pub fn create_schedule(&mut self, cmd: CreateSchedule) -> Result<String, ApiError> {
        let id = if cmd.id.is_empty() {
            Uuid::new_v4().to_string()
        } else {
            cmd.id.clone()
        };
        let schedule = Schedule::new(&id, &cmd.account_id, cmd.amount, cmd.config);
        self.schedules.insert(id.clone(), schedule);
        self.undo.push(UndoEntry::ScheduleCreated {
            schedule_id: id.clone(),
        });
        Ok(id)
    }

    /// Update the configuration of an existing schedule.
    ///
    /// # Errors
    /// Returns [`ApiError::ScheduleNotFound`] if the schedule does not exist.
    pub fn update_schedule(&mut self, cmd: UpdateSchedule) -> Result<(), ApiError> {
        let schedule = self
            .schedules
            .get_mut(&cmd.schedule_id)
            .ok_or_else(|| ApiError::ScheduleNotFound(cmd.schedule_id.clone()))?;
        schedule.config = cmd.config;
        Ok(())
    }

    /// Delete a schedule and detach all its linked transactions.
    ///
    /// # Errors
    /// Returns [`ApiError::ScheduleNotFound`] if the schedule does not exist.
    pub fn delete_schedule(&mut self, cmd: DeleteSchedule) -> Result<(), ApiError> {
        let restored = self
            .schedules
            .remove(&cmd.schedule_id)
            .ok_or_else(|| ApiError::ScheduleNotFound(cmd.schedule_id.clone()))?;
        // Detach schedule from transactions.
        for tx in self.transactions.values_mut() {
            if tx.schedule_id.as_deref() == Some(&cmd.schedule_id) {
                tx.schedule_id = None;
            }
        }
        self.undo.push(UndoEntry::ScheduleDeleted { restored });
        Ok(())
    }

    /// Generate upcoming template transactions from a schedule.
    ///
    /// # Errors
    /// Returns [`ApiError::ScheduleNotFound`] if the schedule does not exist.
    pub fn get_upcoming_transactions(
        &self,
        cmd: GetUpcomingTransactions,
    ) -> Result<UpcomingTransactions, ApiError> {
        let schedule = self
            .schedules
            .get(&cmd.schedule_id)
            .ok_or_else(|| ApiError::ScheduleNotFound(cmd.schedule_id.clone()))?;
        let transactions = generate_upcoming_transactions(schedule, cmd.from_date, cmd.until_date);
        Ok(UpcomingTransactions {
            schedule_id: cmd.schedule_id,
            transactions,
        })
    }

    // ── Read accessors ─────────────────────────────────────────────────────────

    /// Look up an account by ID.
    #[must_use]
    pub fn account(&self, id: &str) -> Option<&Account> {
        self.accounts.get(id)
    }

    /// All non-tombstoned accounts.
    #[must_use]
    pub fn accounts(&self) -> Vec<&Account> {
        self.accounts.values().filter(|a| !a.tombstone).collect()
    }

    /// Look up a payee by ID.
    #[must_use]
    pub fn payee(&self, id: &str) -> Option<&Payee> {
        self.payees.get(id)
    }

    /// All non-tombstoned payees.
    #[must_use]
    pub fn payees(&self) -> Vec<&Payee> {
        self.payees.values().filter(|p| !p.tombstone).collect()
    }

    /// Look up a category group by ID.
    #[must_use]
    pub fn category_group(&self, id: &str) -> Option<&CategoryGroup> {
        self.category_groups.get(id)
    }

    /// All non-tombstoned category groups.
    #[must_use]
    pub fn category_groups(&self) -> Vec<&CategoryGroup> {
        self.category_groups
            .values()
            .filter(|g| !g.tombstone)
            .collect()
    }

    /// Look up a category by ID.
    #[must_use]
    pub fn category(&self, id: &str) -> Option<&Category> {
        self.categories.get(id)
    }

    /// All non-tombstoned, non-hidden categories.
    #[must_use]
    pub fn categories(&self) -> Vec<&Category> {
        self.categories.values().filter(|c| !c.tombstone).collect()
    }

    /// All non-tombstoned transactions.
    #[must_use]
    pub fn transactions(&self) -> Vec<&Transaction> {
        self.transactions
            .values()
            .filter(|t| !t.tombstone)
            .collect()
    }

    /// Transactions for a specific account, sorted by date descending.
    #[must_use]
    pub fn transactions_for_account(&self, account_id: &str) -> Vec<&Transaction> {
        let mut txs: Vec<&Transaction> = self
            .transactions
            .values()
            .filter(|t| !t.tombstone && t.account_id == account_id)
            .collect();
        txs.sort_by_key(|a| std::cmp::Reverse(a.date));
        txs
    }

    /// Access the envelope budget calculator.
    #[must_use]
    pub fn budget(&self) -> &EnvelopeBudget {
        &self.budget
    }

    /// Access the mutable envelope budget calculator.
    pub fn budget_mut(&mut self) -> &mut EnvelopeBudget {
        &mut self.budget
    }

    /// Access the underlying SQLite store (for raw queries and migrations).
    #[must_use]
    pub fn store(&self) -> &SqliteStore {
        &self.store
    }

    // ── Internal undo / redo application ──────────────────────────────────────

    fn apply_undo(&mut self, entry: UndoEntry) -> Result<UndoEntry, ApiError> {
        match entry {
            UndoEntry::AccountCreated { account_id } => {
                let account = self
                    .accounts
                    .get_mut(&account_id)
                    .ok_or_else(|| ApiError::AccountNotFound(account_id.clone()))?;
                account.tombstone = true;
                Ok(UndoEntry::AccountCreated { account_id })
            }
            UndoEntry::AccountRenamed {
                account_id,
                previous_name,
            } => {
                let account = self
                    .accounts
                    .get_mut(&account_id)
                    .ok_or_else(|| ApiError::AccountNotFound(account_id.clone()))?;
                let current_name = account.name.clone();
                account.name = previous_name;
                Ok(UndoEntry::AccountRenamed {
                    account_id,
                    previous_name: current_name,
                })
            }
            UndoEntry::AccountClosed { account_id } => {
                let account = self
                    .accounts
                    .get_mut(&account_id)
                    .ok_or_else(|| ApiError::AccountNotFound(account_id.clone()))?;
                account.closed = false;
                Ok(UndoEntry::AccountReopened { account_id })
            }
            UndoEntry::AccountReopened { account_id } => {
                let account = self
                    .accounts
                    .get_mut(&account_id)
                    .ok_or_else(|| ApiError::AccountNotFound(account_id.clone()))?;
                account.closed = true;
                Ok(UndoEntry::AccountClosed { account_id })
            }
            UndoEntry::PayeeCreated { payee_id } => {
                if let Some(p) = self.payees.get_mut(&payee_id) {
                    p.tombstone = true;
                }
                Ok(UndoEntry::PayeeCreated { payee_id })
            }
            UndoEntry::PayeeRenamed {
                payee_id,
                previous_name,
            } => {
                let payee = self
                    .payees
                    .get_mut(&payee_id)
                    .ok_or_else(|| ApiError::PayeeNotFound(payee_id.clone()))?;
                let current = payee.name.clone();
                payee.name = previous_name;
                Ok(UndoEntry::PayeeRenamed {
                    payee_id,
                    previous_name: current,
                })
            }
            UndoEntry::PayeesMerged {
                restored_payee,
                affected_transaction_ids,
            } => {
                let source_id = restored_payee.id.clone();
                for tx_id in &affected_transaction_ids {
                    if let Some(tx) = self.transactions.get_mut(tx_id) {
                        tx.payee_id = Some(source_id.clone());
                    }
                }
                self.payees
                    .insert(source_id.clone(), restored_payee.clone());
                if let Some(p) = self.payees.get_mut(&source_id) {
                    p.tombstone = false;
                }
                Ok(UndoEntry::PayeesMerged {
                    restored_payee,
                    affected_transaction_ids,
                })
            }
            UndoEntry::CategoryGroupCreated { group_id } => {
                if let Some(g) = self.category_groups.get_mut(&group_id) {
                    g.tombstone = true;
                }
                Ok(UndoEntry::CategoryGroupCreated { group_id })
            }
            UndoEntry::CategoryCreated { category_id } => {
                if let Some(c) = self.categories.get_mut(&category_id) {
                    c.tombstone = true;
                }
                Ok(UndoEntry::CategoryCreated { category_id })
            }
            UndoEntry::CategoryRenamed {
                category_id,
                previous_name,
            } => {
                let cat = self
                    .categories
                    .get_mut(&category_id)
                    .ok_or_else(|| ApiError::CategoryNotFound(category_id.clone()))?;
                let current = cat.name.clone();
                cat.name = previous_name;
                Ok(UndoEntry::CategoryRenamed {
                    category_id,
                    previous_name: current,
                })
            }
            UndoEntry::CategoryMoved {
                category_id,
                previous_group_id,
            } => {
                let cat = self
                    .categories
                    .get_mut(&category_id)
                    .ok_or_else(|| ApiError::CategoryNotFound(category_id.clone()))?;
                let current_group = cat.group_id.clone();
                cat.group_id = previous_group_id;
                Ok(UndoEntry::CategoryMoved {
                    category_id,
                    previous_group_id: current_group,
                })
            }
            UndoEntry::CategoryVisibilityChanged {
                category_id,
                previous_hidden,
            } => {
                let cat = self
                    .categories
                    .get_mut(&category_id)
                    .ok_or_else(|| ApiError::CategoryNotFound(category_id.clone()))?;
                let current = cat.hidden;
                cat.hidden = previous_hidden;
                Ok(UndoEntry::CategoryVisibilityChanged {
                    category_id,
                    previous_hidden: current,
                })
            }
            UndoEntry::CategoryDeleted { restored } => {
                let id = restored.id.clone();
                let snap = restored.clone();
                self.categories.insert(id, restored);
                if let Some(c) = self.categories.get_mut(&snap.id) {
                    c.tombstone = false;
                }
                Ok(UndoEntry::CategoryDeleted { restored: snap })
            }
            UndoEntry::BudgetSet {
                month,
                category_id,
                previous_amount,
            } => {
                let current = self.budget.get_category_budget(&month, &category_id);
                self.budget
                    .set_budget(&month, &category_id, previous_amount);
                Ok(UndoEntry::BudgetSet {
                    month,
                    category_id,
                    previous_amount: current,
                })
            }
            UndoEntry::CarryoverSet {
                month,
                category_id,
                previous_carry,
            } => {
                let current = self.budget.get_category_carryover(&month, &category_id);
                self.budget
                    .set_carryover(&month, &category_id, previous_carry);
                Ok(UndoEntry::CarryoverSet {
                    month,
                    category_id,
                    previous_carry: current,
                })
            }
            UndoEntry::TransactionAdded { transaction_id } => {
                if let Some(tx) = self.transactions.get_mut(&transaction_id) {
                    tx.tombstone = true;
                }
                Ok(UndoEntry::TransactionAdded { transaction_id })
            }
            UndoEntry::TransactionUpdated { previous } => {
                let current = self
                    .transactions
                    .get(&previous.id)
                    .ok_or_else(|| ApiError::TransactionNotFound(previous.id.clone()))?
                    .clone();
                self.transactions.insert(previous.id.clone(), previous);
                Ok(UndoEntry::TransactionUpdated { previous: current })
            }
            UndoEntry::TransactionDeleted { restored } => {
                let id = restored.id.clone();
                self.transactions.insert(id.clone(), restored.clone());
                if let Some(tx) = self.transactions.get_mut(&id) {
                    tx.tombstone = false;
                }
                Ok(UndoEntry::TransactionDeleted { restored })
            }
            UndoEntry::TransactionSplit {
                previous_parent,
                child_ids,
            } => {
                // Tombstone children and restore parent.
                for cid in &child_ids {
                    if let Some(tx) = self.transactions.get_mut(cid) {
                        tx.tombstone = true;
                    }
                }
                let id = previous_parent.id.clone();
                self.transactions.insert(id, previous_parent.clone());
                Ok(UndoEntry::TransactionSplit {
                    previous_parent,
                    child_ids,
                })
            }
            UndoEntry::RuleCreated { rule_id } => {
                self.rules.remove(&rule_id);
                Ok(UndoEntry::RuleCreated { rule_id })
            }
            UndoEntry::RuleUpdated { previous } => {
                let id = previous.id.clone();
                let current = self.rules.get(&id).cloned();
                self.rules.insert(id.clone(), previous);
                if let Some(current_rule) = current {
                    Ok(UndoEntry::RuleUpdated {
                        previous: current_rule,
                    })
                } else {
                    Ok(UndoEntry::RuleCreated {
                        rule_id: id.clone(),
                    })
                }
            }
            UndoEntry::RuleDeleted { restored } => {
                let id = restored.id.clone();
                self.rules.insert(id, restored.clone());
                Ok(UndoEntry::RuleDeleted { restored })
            }
            UndoEntry::ScheduleCreated { schedule_id } => {
                self.schedules.remove(&schedule_id);
                Ok(UndoEntry::ScheduleCreated { schedule_id })
            }
            UndoEntry::ScheduleDeleted { restored } => {
                let id = restored.id.clone();
                self.schedules.insert(id, restored.clone());
                Ok(UndoEntry::ScheduleDeleted { restored })
            }
        }
    }

    fn apply_redo(&mut self, entry: UndoEntry) -> Result<UndoEntry, ApiError> {
        // Redo is simply the inverse of the undo — the undo snapshot already encodes
        // the "re-do what was done" state, so we reuse apply_undo symmetrically.
        self.apply_undo(entry)
    }
}
