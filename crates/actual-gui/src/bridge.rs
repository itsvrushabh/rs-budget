//! Reactive service bridge connecting `BudgetService` to GUI view models.

use crate::error::GuiError;
use crate::table::CellValue;
use crate::views::{BudgetView, TransactionView};
use actual_api::commands::{SetBudget, SetCarryover};
use actual_api::BudgetService;
use actual_core::cents::Cents;

/// Coordinates bidirectional data flow between `BudgetService` domain engine and GUI views.
pub struct GuiBridge {
    pub service: BudgetService,
}

impl GuiBridge {
    /// Creates a new bridge around a `BudgetService`.
    pub fn new(service: BudgetService) -> Self {
        Self { service }
    }

    /// Populates a `BudgetView` from the active budget state.
    pub fn refresh_budget_view(&self, view: &mut BudgetView) -> Result<(), GuiError> {
        view.table.clear();

        let month = view.current_month.clone();
        let budget = self.service.budget();

        let mut total_income = Cents::ZERO;
        let mut total_budgeted = Cents::ZERO;
        let mut total_activity = Cents::ZERO;

        for cat in self.service.categories() {
            let budgeted = budget.get_category_budget(&month, &cat.id);
            let activity = budget.get_category_spent(&month, &cat.id);
            let balance = budget.get_category_leftover(&month, &cat.id);
            let carryover = budget.get_category_carryover(&month, &cat.id);

            if cat.is_income {
                total_income += activity;
            } else {
                total_budgeted += budgeted;
                total_activity += activity;
            }

            view.add_category_row(&cat.id, &cat.name, budgeted, activity, balance, carryover);
        }

        view.summary.total_income = total_income;
        view.summary.total_budgeted = total_budgeted;
        view.summary.total_activity = total_activity;
        view.summary.to_budget = total_income - total_budgeted;
        view.summary.net_balance = total_income + total_activity;

        Ok(())
    }

    /// Populates a `TransactionView` from the active budget state.
    pub fn refresh_transaction_view(&self, view: &mut TransactionView) -> Result<(), GuiError> {
        view.table.clear();

        let txs = match &view.account_id {
            Some(acc_id) => self.service.transactions_for_account(acc_id),
            None => self.service.transactions(),
        };

        for tx in txs {
            // Apply filter query if any
            if !view.filter_query.is_empty() {
                let q = view.filter_query.to_lowercase();
                let matches_notes = tx
                    .notes
                    .as_deref()
                    .unwrap_or("")
                    .to_lowercase()
                    .contains(&q);
                let matches_amount = tx.amount.to_decimal_string(2).contains(&q);
                if !matches_notes && !matches_amount {
                    continue;
                }
            }

            let acc_name = self
                .service
                .account(&tx.account_id)
                .map(|a| a.name.as_str())
                .unwrap_or("Unknown Account");

            let payee_name = tx
                .payee_id
                .as_deref()
                .and_then(|id| self.service.payee(id))
                .map(|p| p.name.as_str())
                .unwrap_or("");

            let cat_name = tx
                .category_id
                .as_deref()
                .and_then(|id| self.service.category(id))
                .map(|c| c.name.as_str())
                .unwrap_or("");

            let date_str = tx.date.to_string();

            view.add_transaction_row(
                &tx.id,
                &date_str,
                acc_name,
                payee_name,
                cat_name,
                tx.notes.as_deref().unwrap_or(""),
                tx.amount,
                tx.cleared,
            );
        }

        Ok(())
    }

    /// Dispatches a committed budget table edit into `BudgetService`.
    pub fn commit_budget_cell(
        &mut self,
        view: &mut BudgetView,
        row: usize,
        col: usize,
    ) -> Result<(), GuiError> {
        let cell = view.table.get_cell_mut(row, col)?;
        let new_value = cell
            .commit_editing()
            .map_err(|reason| GuiError::ValidationFailed { row, col, reason })?;

        let cat_id = view
            .table
            .row_ids
            .get(row)
            .ok_or(GuiError::OutOfBounds(row, col))?
            .clone();

        let month = view.current_month.clone();

        match (col, new_value) {
            (1, CellValue::Currency(amount)) => {
                // Column 1 is Budgeted amount
                self.service
                    .set_budget(SetBudget {
                        category_id: cat_id,
                        month,
                        amount,
                    })
                    .map_err(|e| GuiError::Service(e.to_string()))?;
            }
            (4, CellValue::Boolean(carryover)) => {
                // Column 4 is Carryover toggle
                self.service
                    .set_carryover(SetCarryover {
                        category_id: cat_id,
                        month,
                        carry: carryover,
                    })
                    .map_err(|e| GuiError::Service(e.to_string()))?;
            }
            _ => {}
        }

        // Recompute balances
        self.refresh_budget_view(view)?;

        Ok(())
    }

    /// Triggers an undo operation through the domain service.
    pub fn undo(&mut self, view: &mut BudgetView) -> Result<bool, GuiError> {
        if self.service.undo_depth() > 0 {
            self.service
                .undo()
                .map_err(|e| GuiError::Service(e.to_string()))?;
            self.refresh_budget_view(view)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Triggers a redo operation through the domain service.
    pub fn redo(&mut self, view: &mut BudgetView) -> Result<bool, GuiError> {
        if self.service.redo_depth() > 0 {
            self.service
                .redo()
                .map_err(|e| GuiError::Service(e.to_string()))?;
            self.refresh_budget_view(view)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}
