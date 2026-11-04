//! Web application state orchestrator.
//!
//! Coordinates OPFS storage, active budget files, web routes, sync bridge,
//! and view models for the browser WebAssembly client.

use crate::web::opfs::{OpfsError, OpfsStorageBackend};
use crate::web::sync_bridge::{OnlineState, WebSyncBridge, WebSyncConfig, WebSyncState};
use actual_core::cents::Cents;
use actual_core::models::{Account, Category, CategoryGroup, Transaction};
use actual_sync::Timestamp;
use serde::{Deserialize, Serialize};

/// Navigation routes within the Web client.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WebRoute {
    Budget,
    Accounts,
    Transactions,
    Reports,
    Settings,
}

/// A budget entry record managed in the web client.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebBudgetMetadata {
    pub id: String,
    pub name: String,
    pub file_path: String,
    pub last_opened_unix: u64,
}

/// Top-level Web application state.
pub struct WebApp {
    pub storage: OpfsStorageBackend,
    pub sync_bridge: WebSyncBridge,
    pub current_route: WebRoute,
    pub current_month: String,
    pub active_budget: Option<WebBudgetMetadata>,
    pub category_groups: Vec<CategoryGroup>,
    pub categories: Vec<Category>,
    pub accounts: Vec<Account>,
    pub transactions: Vec<Transaction>,
}

impl WebApp {
    /// Creates a new WebApp with OPFS root prefix.
    pub fn new(root_prefix: &str, sync_config: WebSyncConfig) -> Self {
        Self {
            storage: OpfsStorageBackend::new(root_prefix),
            sync_bridge: WebSyncBridge::new(sync_config),
            current_route: WebRoute::Budget,
            current_month: "2026-11".to_string(),
            active_budget: None,
            category_groups: Vec::new(),
            categories: Vec::new(),
            accounts: Vec::new(),
            transactions: Vec::new(),
        }
    }

    /// Initializes a default budget in OPFS.
    pub fn create_budget(
        &mut self,
        id: &str,
        name: &str,
        timestamp_unix: u64,
    ) -> Result<(), OpfsError> {
        let file_path = format!("budgets/{id}.actual");
        self.storage.create_file(&file_path, timestamp_unix)?;

        let meta = WebBudgetMetadata {
            id: id.to_string(),
            name: name.to_string(),
            file_path,
            last_opened_unix: timestamp_unix,
        };
        self.active_budget = Some(meta);

        // Seed default category group and category
        let group_id = "group-general";
        let group = CategoryGroup::new(group_id, "General");
        let cat = Category::new("cat-groceries", group_id, "Groceries");

        self.category_groups = vec![group];
        self.categories = vec![cat];
        self.accounts.clear();
        self.transactions.clear();

        Ok(())
    }

    /// Saves the active budget database/archive into OPFS.
    pub fn save_active_budget(&mut self, timestamp_unix: u64) -> Result<(), OpfsError> {
        let path = match &self.active_budget {
            Some(meta) => meta.file_path.clone(),
            None => return Err(OpfsError::NotFound("No active budget to save".into())),
        };

        // Serialize state summary to payload
        let state_json = serde_json::json!({
            "groups": self.category_groups,
            "categories": self.categories,
            "accounts": self.accounts,
            "transaction_count": self.transactions.len(),
            "updated_at": timestamp_unix,
        });
        let bytes = serde_json::to_vec(&state_json).unwrap_or_default();
        self.storage.write_file(&path, &bytes, timestamp_unix)?;
        Ok(())
    }

    /// Loads a budget from OPFS by path.
    pub fn load_budget(&mut self, file_path: &str, timestamp_unix: u64) -> Result<(), OpfsError> {
        let bytes = self.storage.read_file(file_path)?;
        if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&bytes) {
            if let Some(groups) = val.get("groups") {
                if let Ok(parsed) = serde_json::from_value::<Vec<CategoryGroup>>(groups.clone()) {
                    self.category_groups = parsed;
                }
            }
            if let Some(cats) = val.get("categories") {
                if let Ok(parsed) = serde_json::from_value::<Vec<Category>>(cats.clone()) {
                    self.categories = parsed;
                }
            }
            if let Some(accs) = val.get("accounts") {
                if let Ok(parsed) = serde_json::from_value::<Vec<Account>>(accs.clone()) {
                    self.accounts = parsed;
                }
            }
        }

        let id = file_path
            .trim_start_matches("budgets/")
            .trim_end_matches(".actual");
        self.active_budget = Some(WebBudgetMetadata {
            id: id.to_string(),
            name: id.to_string(),
            file_path: file_path.to_string(),
            last_opened_unix: timestamp_unix,
        });
        Ok(())
    }

    /// Navigates to a specific route.
    pub fn navigate_to(&mut self, route: WebRoute) {
        self.current_route = route;
    }

    /// Adds an account and enqueues sync change.
    pub fn add_account(&mut self, id: &str, name: &str, offbudget: bool) {
        let mut account = Account::new(id, name);
        account.offbudget = offbudget;
        self.accounts.push(account);

        // Queue change
        let ts = Timestamp::new(1700000000000, 0, "webclient0000001");
        let crdt_msg = actual_sync::CrdtMessage::new(ts, "accounts", id, "name", name);
        self.sync_bridge.enqueue_change(crdt_msg);
    }

    /// Adds a transaction and enqueues sync change.
    pub fn add_transaction(&mut self, tx: Transaction) {
        let ts = Timestamp::new(1700000000001, 0, "webclient0000001");
        let crdt_msg = actual_sync::CrdtMessage::new(
            ts,
            "transactions",
            &tx.id,
            "amount",
            tx.amount.0.to_string(),
        );
        self.transactions.push(tx);
        self.sync_bridge.enqueue_change(crdt_msg);
    }

    /// Calculates total balance of all active accounts.
    pub fn total_balance(&self) -> Cents {
        let mut sum = Cents::ZERO;
        for tx in &self.transactions {
            if !tx.is_parent {
                sum += tx.amount;
            }
        }
        sum
    }

    /// Updates network status and auto-flushes if back online.
    pub fn update_network_status(&mut self, state: OnlineState, timestamp_unix: u64) {
        self.sync_bridge.set_online_state(state);
        if state == OnlineState::Online && self.sync_bridge.pending_queue_size() > 0 {
            let _ = self.sync_bridge.flush_offline_queue(timestamp_unix);
        }
    }

    /// Returns current sync status badge.
    pub fn sync_status(&self) -> WebSyncState {
        self.sync_bridge.sync_state
    }
}
