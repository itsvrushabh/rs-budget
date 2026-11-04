//! Application state and navigation models for the interactive TUI.

use actual_api::commands::{
    AddTransaction, CreateAccount, CreateCategory, CreateCategoryGroup, SetBudget,
};
use actual_api::BudgetService;
use actual_core::{Cents, Transaction};
use chrono::{Datelike, Local, NaiveDate};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveView {
    Budget,
    Accounts,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveModal {
    None,
    Help,
    EditBudget,
    AddTransaction,
}

/// Item in the flattened budget view (either a group header or a category row).
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum BudgetItem {
    Group {
        id: String,
        name: String,
        total_budgeted: Cents,
        total_spent: Cents,
        total_leftover: Cents,
    },
    Category {
        id: String,
        name: String,
        group_id: String,
        budgeted: Cents,
        spent: Cents,
        leftover: Cents,
        carryover: bool,
    },
}

pub struct App {
    pub svc: BudgetService,
    pub active_view: ActiveView,
    pub active_modal: ActiveModal,
    pub current_month: NaiveDate, // First of current month

    // Navigation selections
    pub budget_cursor: usize,
    #[allow(dead_code)]
    pub account_cursor: usize,
    pub transaction_cursor: usize,
    pub selected_account_id: Option<String>,

    // Search filter
    pub search_query: String,
    pub search_active: bool,

    // Edit budget modal state
    pub edit_category_id: String,
    pub edit_category_name: String,
    pub edit_amount_input: String,

    // Add transaction modal state
    pub add_tx_amount: String,
    pub add_tx_payee: String,
    pub add_tx_category: String,
    pub add_tx_notes: String,
    pub add_tx_field_idx: usize, // 0=Amount, 1=Payee, 2=Category, 3=Notes

    pub status_message: Option<String>,
    pub should_quit: bool,
}

impl App {
    #[must_use]
    pub fn new(svc: BudgetService) -> Self {
        let now = Local::now().date_naive();
        let current_month = NaiveDate::from_ymd_opt(now.year(), now.month(), 1).unwrap();

        let mut app = Self {
            svc,
            active_view: ActiveView::Budget,
            active_modal: ActiveModal::None,
            current_month,
            budget_cursor: 0,
            account_cursor: 0,
            transaction_cursor: 0,
            selected_account_id: None,
            search_query: String::new(),
            search_active: false,
            edit_category_id: String::new(),
            edit_category_name: String::new(),
            edit_amount_input: String::new(),
            add_tx_amount: String::new(),
            add_tx_payee: String::new(),
            add_tx_category: String::new(),
            add_tx_notes: String::new(),
            add_tx_field_idx: 0,
            status_message: None,
            should_quit: false,
        };

        // Seed sample categories if empty
        if app.svc.categories().is_empty() {
            app.seed_default_categories();
        }

        app
    }

    pub fn current_month_str(&self) -> String {
        self.current_month.format("%Y-%m").to_string()
    }

    pub fn prev_month(&mut self) {
        let y = self.current_month.year();
        let m = self.current_month.month();
        self.current_month = if m == 1 {
            NaiveDate::from_ymd_opt(y - 1, 12, 1).unwrap()
        } else {
            NaiveDate::from_ymd_opt(y, m - 1, 1).unwrap()
        };
        self.budget_cursor = 0;
    }

    pub fn next_month(&mut self) {
        let y = self.current_month.year();
        let m = self.current_month.month();
        self.current_month = if m == 12 {
            NaiveDate::from_ymd_opt(y + 1, 1, 1).unwrap()
        } else {
            NaiveDate::from_ymd_opt(y, m + 1, 1).unwrap()
        };
        self.budget_cursor = 0;
    }

    /// Builds the flattened list of category groups and envelope categories.
    pub fn build_budget_items(&self) -> Vec<BudgetItem> {
        let m = self.current_month_str();
        let mut items = Vec::new();

        for grp in self.svc.category_groups() {
            let cats: Vec<_> = self
                .svc
                .categories()
                .into_iter()
                .filter(|c| c.group_id == grp.id)
                .collect();

            let grp_budgeted = self.svc.budget().get_group_budget(&m, &grp.id);
            let grp_spent = self.svc.budget().get_group_spent(&m, &grp.id);
            let grp_leftover = self.svc.budget().get_group_leftover(&m, &grp.id);

            items.push(BudgetItem::Group {
                id: grp.id.clone(),
                name: grp.name.clone(),
                total_budgeted: grp_budgeted,
                total_spent: grp_spent,
                total_leftover: grp_leftover,
            });

            for cat in cats {
                let b = self.svc.budget().get_category_budget(&m, &cat.id);
                let s = self.svc.budget().get_category_spent(&m, &cat.id);
                let l = self.svc.budget().get_category_leftover(&m, &cat.id);
                let c = self.svc.budget().get_category_carryover(&m, &cat.id);

                items.push(BudgetItem::Category {
                    id: cat.id.clone(),
                    name: cat.name.clone(),
                    group_id: grp.id.clone(),
                    budgeted: b,
                    spent: s,
                    leftover: l,
                    carryover: c,
                });
            }
        }

        items
    }

    /// Returns filtered transactions for the currently selected account.
    pub fn filtered_transactions(&self) -> Vec<&Transaction> {
        let txs = if let Some(ref aid) = self.selected_account_id {
            self.svc.transactions_for_account(aid)
        } else {
            let mut all = self.svc.transactions();
            all.sort_by_key(|t| std::cmp::Reverse(t.date));
            all
        };

        if self.search_query.trim().is_empty() {
            txs
        } else {
            let q = self.search_query.to_ascii_lowercase();
            txs.into_iter()
                .filter(|t| {
                    let payee = t.payee_id.as_deref().unwrap_or("").to_ascii_lowercase();
                    let notes = t.notes.as_deref().unwrap_or("").to_ascii_lowercase();
                    payee.contains(&q) || notes.contains(&q)
                })
                .collect()
        }
    }

    /// Open modal to edit selected category's budget allocation.
    pub fn open_edit_budget(&mut self) {
        let items = self.build_budget_items();
        if let Some(BudgetItem::Category {
            id, name, budgeted, ..
        }) = items.get(self.budget_cursor)
        {
            self.edit_category_id = id.clone();
            self.edit_category_name = name.clone();
            let dollars = budgeted.as_cents() as f64 / 100.0;
            self.edit_amount_input = format!("{dollars:.2}");
            self.active_modal = ActiveModal::EditBudget;
        }
    }

    /// Commit the edited budget amount.
    pub fn commit_edit_budget(&mut self) {
        if let Ok(amount) = self.edit_amount_input.trim().parse::<f64>() {
            let cents = Cents::from_cents((amount * 100.0).round() as i64);
            let m = self.current_month_str();
            let _ = self.svc.set_budget(SetBudget {
                month: m,
                category_id: self.edit_category_id.clone(),
                amount: cents,
            });
            self.status_message = Some(format!("Updated budget for {}", self.edit_category_name));
        }
        self.active_modal = ActiveModal::None;
    }

    /// Open modal to record a new transaction.
    pub fn open_add_transaction(&mut self) {
        self.add_tx_amount.clear();
        self.add_tx_payee.clear();
        self.add_tx_category.clear();
        self.add_tx_notes.clear();
        self.add_tx_field_idx = 0;
        self.active_modal = ActiveModal::AddTransaction;
    }

    /// Commit new transaction.
    pub fn commit_add_transaction(&mut self) {
        let account_id = if let Some(ref aid) = self.selected_account_id {
            aid.clone()
        } else if let Some(first_acc) = self.svc.accounts().first() {
            first_acc.id.clone()
        } else {
            // Create default checking account if none exists
            self.svc
                .create_account(CreateAccount {
                    id: String::new(),
                    name: "Checking".into(),
                    offbudget: false,
                })
                .unwrap()
        };

        let amount_f64 = self.add_tx_amount.trim().parse::<f64>().unwrap_or(0.0);
        let cents = Cents::from_cents((amount_f64 * 100.0).round() as i64);

        let payee = if self.add_tx_payee.trim().is_empty() {
            None
        } else {
            Some(self.add_tx_payee.trim().to_string())
        };

        let notes = if self.add_tx_notes.trim().is_empty() {
            None
        } else {
            Some(self.add_tx_notes.trim().to_string())
        };

        // Match category by name
        let cat_name = self.add_tx_category.trim();
        let cat_id = self
            .svc
            .categories()
            .into_iter()
            .find(|c| c.name.eq_ignore_ascii_case(cat_name))
            .map(|c| c.id.clone());

        let cmd = AddTransaction {
            id: String::new(),
            account_id,
            date: Local::now().date_naive(),
            amount: cents,
            payee_id: payee,
            category_id: cat_id,
            notes,
            cleared: true,
        };

        if let Ok(id) = self.svc.add_transaction(cmd) {
            self.status_message = Some(format!("Added transaction {id}"));
        }

        self.active_modal = ActiveModal::None;
    }

    fn seed_default_categories(&mut self) {
        let g1 = self
            .svc
            .create_category_group(CreateCategoryGroup {
                id: "grp-living".into(),
                name: "Living Expenses".into(),
                is_income: false,
            })
            .unwrap();

        let _ = self.svc.create_category(CreateCategory {
            id: "cat-rent".into(),
            name: "Rent / Mortgage".into(),
            group_id: g1.clone(),
            is_income: false,
        });

        let _ = self.svc.create_category(CreateCategory {
            id: "cat-groceries".into(),
            name: "Groceries".into(),
            group_id: g1,
            is_income: false,
        });

        let g2 = self
            .svc
            .create_category_group(CreateCategoryGroup {
                id: "grp-discretionary".into(),
                name: "Discretionary".into(),
                is_income: false,
            })
            .unwrap();

        let _ = self.svc.create_category(CreateCategory {
            id: "cat-dining".into(),
            name: "Dining Out".into(),
            group_id: g2.clone(),
            is_income: false,
        });

        let _ = self.svc.create_category(CreateCategory {
            id: "cat-entertainment".into(),
            name: "Entertainment".into(),
            group_id: g2,
            is_income: false,
        });
    }
}
