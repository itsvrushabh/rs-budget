//! In-memory undo / redo stack for reversible budget service commands.
//!
//! The undo manager stores "inverse snapshots" — the minimal data required to
//! reverse a completed command — rather than re-running full commands.

use crate::error::ApiError;
use actual_core::{Account, Category, Payee, Rule, Transaction};

/// The inverse (undo) payload for each reversible operation.
///
/// Each variant carries whatever is needed to restore pre-command state.
#[derive(Debug, Clone)]
pub enum UndoEntry {
    /// Undo CreateAccount — tombstone the account.
    AccountCreated { account_id: String },
    /// Undo RenameAccount — restore previous name.
    AccountRenamed {
        account_id: String,
        previous_name: String,
    },
    /// Undo CloseAccount — reopen the account.
    AccountClosed { account_id: String },
    /// Undo ReopenAccount — close the account again.
    AccountReopened { account_id: String },

    /// Undo CreatePayee — tombstone the payee.
    PayeeCreated { payee_id: String },
    /// Undo RenamePayee — restore previous name.
    PayeeRenamed {
        payee_id: String,
        previous_name: String,
    },
    /// Undo MergePayees — restore source payee and re-point its transactions.
    PayeesMerged {
        restored_payee: Payee,
        affected_transaction_ids: Vec<String>,
    },

    /// Undo CreateCategoryGroup — tombstone the group.
    CategoryGroupCreated { group_id: String },
    /// Undo CreateCategory — tombstone the category.
    CategoryCreated { category_id: String },
    /// Undo RenameCategory — restore previous name.
    CategoryRenamed {
        category_id: String,
        previous_name: String,
    },
    /// Undo MoveCategory — restore previous group membership.
    CategoryMoved {
        category_id: String,
        previous_group_id: String,
    },
    /// Undo SetCategoryVisibility — restore previous visibility.
    CategoryVisibilityChanged {
        category_id: String,
        previous_hidden: bool,
    },
    /// Undo DeleteCategory — restore full category snapshot.
    CategoryDeleted { restored: Category },

    /// Undo SetBudget — restore previous budgeted amount.
    BudgetSet {
        month: String,
        category_id: String,
        previous_amount: actual_core::Cents,
    },
    /// Undo SetCarryover — restore previous carryover flag.
    CarryoverSet {
        month: String,
        category_id: String,
        previous_carry: bool,
    },

    /// Undo AddTransaction — tombstone the transaction.
    TransactionAdded { transaction_id: String },
    /// Undo UpdateTransaction — restore the previous transaction snapshot.
    TransactionUpdated { previous: Transaction },
    /// Undo DeleteTransaction — restore the transaction snapshot.
    TransactionDeleted { restored: Transaction },
    /// Undo SplitTransaction — restore the pre-split transaction and remove children.
    TransactionSplit {
        previous_parent: Transaction,
        child_ids: Vec<String>,
    },

    /// Undo CreateRule — remove the rule.
    RuleCreated { rule_id: String },
    /// Undo UpdateRule — restore the previous rule.
    RuleUpdated { previous: Rule },
    /// Undo DeleteRule — restore the full rule snapshot.
    RuleDeleted { restored: Rule },

    /// Undo CreateSchedule — remove the schedule.
    ScheduleCreated { schedule_id: String },
    /// Undo DeleteSchedule — restore the schedule config.
    ScheduleDeleted { restored: actual_core::Schedule },
}

/// Bidirectional undo / redo history.
///
/// `undo_stack` holds past operations (top = most recent).
/// `redo_stack` holds reverted operations (top = most recently undone).
/// Both stacks are capped at [`UndoManager::MAX_DEPTH`].
#[derive(Debug, Default)]
pub struct UndoManager {
    undo_stack: Vec<UndoEntry>,
    redo_stack: Vec<UndoEntry>,
}

impl UndoManager {
    /// Maximum number of operations tracked in each direction.
    pub const MAX_DEPTH: usize = 200;

    /// Create a new empty `UndoManager`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a completed operation.
    ///
    /// Clears the redo stack (a new operation invalidates undone future).
    pub fn push(&mut self, entry: UndoEntry) {
        self.redo_stack.clear();
        if self.undo_stack.len() >= Self::MAX_DEPTH {
            self.undo_stack.remove(0);
        }
        self.undo_stack.push(entry);
    }

    /// Pop the most-recently-executed operation for undoing.
    ///
    /// # Errors
    /// Returns [`ApiError::NothingToUndo`] when the stack is empty.
    pub fn pop_undo(&mut self) -> Result<UndoEntry, ApiError> {
        self.undo_stack.pop().ok_or(ApiError::NothingToUndo)
    }

    /// After a successful undo, record the inverse so it can be re-done.
    pub fn push_redo(&mut self, entry: UndoEntry) {
        if self.redo_stack.len() >= Self::MAX_DEPTH {
            self.redo_stack.remove(0);
        }
        self.redo_stack.push(entry);
    }

    /// Pop the most-recently-undone operation for redoing.
    ///
    /// # Errors
    /// Returns [`ApiError::NothingToRedo`] when the stack is empty.
    pub fn pop_redo(&mut self) -> Result<UndoEntry, ApiError> {
        self.redo_stack.pop().ok_or(ApiError::NothingToRedo)
    }

    /// Number of undoable operations.
    #[must_use]
    pub fn undo_depth(&self) -> usize {
        self.undo_stack.len()
    }

    /// Number of redoable operations.
    #[must_use]
    pub fn redo_depth(&self) -> usize {
        self.redo_stack.len()
    }

    /// Clear both stacks.
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    // ── helpers used by BudgetService ──────────────────────────────────────

    /// Snapshot an [`Account`] into an undo entry restoring a rename.
    #[must_use]
    pub fn snapshot_account_rename(account: &Account) -> UndoEntry {
        UndoEntry::AccountRenamed {
            account_id: account.id.clone(),
            previous_name: account.name.clone(),
        }
    }

    /// Snapshot a [`Payee`] into an undo entry restoring a rename.
    #[must_use]
    pub fn snapshot_payee_rename(payee: &Payee) -> UndoEntry {
        UndoEntry::PayeeRenamed {
            payee_id: payee.id.clone(),
            previous_name: payee.name.clone(),
        }
    }

    /// Snapshot a [`Category`] into an undo entry restoring a rename.
    #[must_use]
    pub fn snapshot_category_rename(category: &Category) -> UndoEntry {
        UndoEntry::CategoryRenamed {
            category_id: category.id.clone(),
            previous_name: category.name.clone(),
        }
    }
}
