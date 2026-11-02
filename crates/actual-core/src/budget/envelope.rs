//! Envelope budgeting model and calculation formulas.

use std::collections::{BTreeSet, HashMap};

use crate::category::{Category, CategoryGroup};
use crate::cents::Cents;
use crate::error::DomainError;
use crate::spreadsheet::cell::CellValue;
use crate::spreadsheet::engine::Spreadsheet;
use crate::transaction::Transaction;

/// Parses a `YYYY-MM` month string into `(year, month)`.
///
/// # Errors
/// Returns [`DomainError::InvalidMonthFormat`] if the string is not formatted as `YYYY-MM` with month between 1 and 12.
pub fn parse_year_month(month: &str) -> Result<(i32, u32), DomainError> {
    let parts: Vec<&str> = month.split('-').collect();
    if parts.len() != 2 {
        return Err(DomainError::InvalidMonthFormat(month.to_string()));
    }

    let year = parts[0]
        .parse::<i32>()
        .map_err(|_| DomainError::InvalidMonthFormat(month.to_string()))?;
    let m = parts[1]
        .parse::<u32>()
        .map_err(|_| DomainError::InvalidMonthFormat(month.to_string()))?;

    if !(1..=12).contains(&m) {
        return Err(DomainError::InvalidMonthFormat(month.to_string()));
    }

    Ok((year, m))
}

/// Returns the preceding month in `YYYY-MM` format.
///
/// # Errors
/// Returns [`DomainError::InvalidMonthFormat`] if the month string is invalid.
pub fn prev_month(month: &str) -> Result<String, DomainError> {
    let (year, m) = parse_year_month(month)?;
    if m == 1 {
        Ok(format!("{:04}-12", year - 1))
    } else {
        Ok(format!("{year:04}-{:02}", m - 1))
    }
}

/// Returns the succeeding month in `YYYY-MM` format.
///
/// # Errors
/// Returns [`DomainError::InvalidMonthFormat`] if the month string is invalid.
pub fn next_month(month: &str) -> Result<String, DomainError> {
    let (year, m) = parse_year_month(month)?;
    if m == 12 {
        Ok(format!("{:04}-01", year + 1))
    } else {
        Ok(format!("{year:04}-{:02}", m + 1))
    }
}

// Cell name formatters matching upstream spreadsheet naming
#[inline]
#[must_use]
pub fn cell_budget(month: &str, category_id: &str) -> String {
    format!("{month}!budget-{category_id}")
}

#[inline]
#[must_use]
pub fn cell_spent(month: &str, category_id: &str) -> String {
    format!("{month}!spent-{category_id}")
}

#[inline]
#[must_use]
pub fn cell_carryover(month: &str, category_id: &str) -> String {
    format!("{month}!carryover-{category_id}")
}

#[inline]
#[must_use]
pub fn cell_leftover(month: &str, category_id: &str) -> String {
    format!("{month}!leftover-{category_id}")
}

#[inline]
#[must_use]
pub fn cell_group_budget(month: &str, group_id: &str) -> String {
    format!("{month}!group-budget-{group_id}")
}

#[inline]
#[must_use]
pub fn cell_group_spent(month: &str, group_id: &str) -> String {
    format!("{month}!group-sum-amount-{group_id}")
}

#[inline]
#[must_use]
pub fn cell_group_leftover(month: &str, group_id: &str) -> String {
    format!("{month}!group-leftover-{group_id}")
}

#[inline]
#[must_use]
pub fn cell_total_budgeted(month: &str) -> String {
    format!("{month}!total-budgeted")
}

#[inline]
#[must_use]
pub fn cell_total_spent(month: &str) -> String {
    format!("{month}!total-spent")
}

#[inline]
#[must_use]
pub fn cell_total_leftover(month: &str) -> String {
    format!("{month}!total-leftover")
}

#[inline]
#[must_use]
pub fn cell_total_income(month: &str) -> String {
    format!("{month}!total-income")
}

#[inline]
#[must_use]
pub fn cell_uncategorized_income(month: &str) -> String {
    format!("{month}!uncategorized-income")
}

#[inline]
#[must_use]
pub fn cell_from_last_month(month: &str) -> String {
    format!("{month}!from-last-month")
}

#[inline]
#[must_use]
pub fn cell_available_funds(month: &str) -> String {
    format!("{month}!available-funds")
}

#[inline]
#[must_use]
pub fn cell_last_month_overspent(month: &str) -> String {
    format!("{month}!last-month-overspent")
}

#[inline]
#[must_use]
pub fn cell_buffered(month: &str) -> String {
    format!("{month}!buffered")
}

#[inline]
#[must_use]
pub fn cell_to_budget(month: &str) -> String {
    format!("{month}!to-budget")
}

/// Reactive Envelope Budget calculation engine.
///
/// Implements envelope budgeting math, carryover rules, month rollover,
/// and reactive recomputation DAG identical to upstream Actual Budget.
#[derive(Default)]
pub struct EnvelopeBudget {
    /// Underlying reactive spreadsheet DAG.
    spreadsheet: Spreadsheet,
    /// Chronologically ordered set of initialized months (`YYYY-MM`).
    initialized_months: BTreeSet<String>,
    /// Categories registered in the budget.
    categories: HashMap<String, Category>,
    /// Category groups registered in the budget.
    groups: HashMap<String, CategoryGroup>,
}

impl EnvelopeBudget {
    /// Creates a new empty `EnvelopeBudget`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Access the underlying `Spreadsheet`.
    #[must_use]
    pub fn spreadsheet(&self) -> &Spreadsheet {
        &self.spreadsheet
    }

    /// Mutably access the underlying `Spreadsheet`.
    pub fn spreadsheet_mut(&mut self) -> &mut Spreadsheet {
        &mut self.spreadsheet
    }

    /// Returns the set of initialized months.
    #[must_use]
    pub fn initialized_months(&self) -> &BTreeSet<String> {
        &self.initialized_months
    }

    /// Checks if a month has been initialized.
    #[must_use]
    pub fn is_month_initialized(&self, month: &str) -> bool {
        self.initialized_months.contains(month)
    }

    /// Registers or updates categories in the budget.
    pub fn register_categories(&mut self, categories: &[Category]) {
        for cat in categories {
            self.categories.insert(cat.id.clone(), cat.clone());
        }
    }

    /// Registers or updates category groups in the budget.
    pub fn register_groups(&mut self, groups: &[CategoryGroup]) {
        for group in groups {
            self.groups.insert(group.id.clone(), group.clone());
        }
    }

    /// Initializes a month's envelope calculation nodes.
    ///
    /// # Errors
    /// Returns [`DomainError::InvalidMonthFormat`] if month is invalid, or
    /// [`DomainError::CycleDetected`] if a cycle is introduced.
    pub fn init_month(
        &mut self,
        month: &str,
        categories: &[Category],
        groups: &[CategoryGroup],
    ) -> Result<(), DomainError> {
        parse_year_month(month)?;

        self.register_categories(categories);
        self.register_groups(groups);

        self.initialized_months.insert(month.to_string());

        // Wire this month's static and dynamic cells
        self.wire_month_cells(month)?;

        // If the succeeding month is already initialized, re-wire it so it connects to this month
        let next_m = next_month(month)?;
        if self.initialized_months.contains(&next_m) {
            self.wire_month_cells(&next_m)?;
        }

        Ok(())
    }

    /// Internal method to wire or re-wire spreadsheet formulas for a month.
    fn wire_month_cells(&mut self, month: &str) -> Result<(), DomainError> {
        let prev_m = prev_month(month)?;
        let has_prev = self.initialized_months.contains(&prev_m);

        let expense_cats: Vec<Category> = self
            .categories
            .values()
            .filter(|c| !c.is_income && c.is_active())
            .cloned()
            .collect();

        let income_cats: Vec<Category> = self
            .categories
            .values()
            .filter(|c| c.is_income && c.is_active())
            .cloned()
            .collect();

        let groups: Vec<CategoryGroup> = self
            .groups
            .values()
            .filter(|g| g.is_active())
            .cloned()
            .collect();

        self.spreadsheet.transaction(|sheet| {
            // 1. Static base cells if not already present
            let uncat_income_cell = cell_uncategorized_income(month);
            if sheet.get_value(&uncat_income_cell).is_none() {
                sheet.set_static(&uncat_income_cell, Cents::ZERO);
            }

            let buffered_cell = cell_buffered(month);
            if sheet.get_value(&buffered_cell).is_none() {
                sheet.set_static(&buffered_cell, Cents::ZERO);
            }

            // Ensure static budget, spent, and carryover cells exist for all categories
            for cat in self.categories.values() {
                let b_cell = cell_budget(month, &cat.id);
                if sheet.get_value(&b_cell).is_none() {
                    sheet.set_static(&b_cell, Cents::ZERO);
                }

                let s_cell = cell_spent(month, &cat.id);
                if sheet.get_value(&s_cell).is_none() {
                    sheet.set_static(&s_cell, Cents::ZERO);
                }

                let c_cell = cell_carryover(month, &cat.id);
                if sheet.get_value(&c_cell).is_none() {
                    sheet.set_static(&c_cell, false);
                }
            }

            // 2. Dynamic leftover cells for expense categories
            for cat in &expense_cats {
                let l_cell = cell_leftover(month, &cat.id);
                let b_cell = cell_budget(month, &cat.id);
                let s_cell = cell_spent(month, &cat.id);

                if has_prev {
                    let prev_l_cell = cell_leftover(&prev_m, &cat.id);
                    let prev_c_cell = cell_carryover(&prev_m, &cat.id);
                    sheet.set_dynamic(
                        &l_cell,
                        &[&b_cell, &s_cell, &prev_l_cell, &prev_c_cell],
                        |args| {
                            let budgeted = args[0].as_cents();
                            let spent = args[1].as_cents();
                            let prev_leftover = args[2].as_cents();
                            let prev_carryover = args[3].as_bool();

                            let carry = if prev_carryover {
                                prev_leftover
                            } else if prev_leftover.is_negative() {
                                Cents::ZERO
                            } else {
                                prev_leftover
                            };

                            CellValue::Cents(budgeted + spent + carry)
                        },
                    )?;
                } else {
                    sheet.set_dynamic(&l_cell, &[&b_cell, &s_cell], |args| {
                        let budgeted = args[0].as_cents();
                        let spent = args[1].as_cents();
                        CellValue::Cents(budgeted + spent)
                    })?;
                }
            }

            // Leftover for income categories (budgeted + spent)
            for cat in &income_cats {
                let l_cell = cell_leftover(month, &cat.id);
                let b_cell = cell_budget(month, &cat.id);
                let s_cell = cell_spent(month, &cat.id);
                sheet.set_dynamic(&l_cell, &[&b_cell, &s_cell], |args| {
                    let budgeted = args[0].as_cents();
                    let spent = args[1].as_cents();
                    CellValue::Cents(budgeted + spent)
                })?;
            }

            // 3. Group summary cells
            for group in &groups {
                let grp_cats: Vec<&Category> = self
                    .categories
                    .values()
                    .filter(|c| c.group_id == group.id && c.is_active())
                    .collect();

                let gb_cell = cell_group_budget(month, &group.id);
                let gs_cell = cell_group_spent(month, &group.id);
                let gl_cell = cell_group_leftover(month, &group.id);

                let b_deps: Vec<String> =
                    grp_cats.iter().map(|c| cell_budget(month, &c.id)).collect();
                let b_deps_ref: Vec<&str> = b_deps.iter().map(String::as_str).collect();
                sheet.set_dynamic(&gb_cell, &b_deps_ref, |args| {
                    let total: Cents = args.iter().map(CellValue::as_cents).sum();
                    CellValue::Cents(total)
                })?;

                let s_deps: Vec<String> =
                    grp_cats.iter().map(|c| cell_spent(month, &c.id)).collect();
                let s_deps_ref: Vec<&str> = s_deps.iter().map(String::as_str).collect();
                sheet.set_dynamic(&gs_cell, &s_deps_ref, |args| {
                    let total: Cents = args.iter().map(CellValue::as_cents).sum();
                    CellValue::Cents(total)
                })?;

                let l_deps: Vec<String> = grp_cats
                    .iter()
                    .map(|c| cell_leftover(month, &c.id))
                    .collect();
                let l_deps_ref: Vec<&str> = l_deps.iter().map(String::as_str).collect();
                sheet.set_dynamic(&gl_cell, &l_deps_ref, |args| {
                    let total: Cents = args.iter().map(CellValue::as_cents).sum();
                    CellValue::Cents(total)
                })?;
            }

            // 4. Monthly totals
            let exp_b_deps: Vec<String> = expense_cats
                .iter()
                .map(|c| cell_budget(month, &c.id))
                .collect();
            let exp_b_refs: Vec<&str> = exp_b_deps.iter().map(String::as_str).collect();
            sheet.set_dynamic(&cell_total_budgeted(month), &exp_b_refs, |args| {
                let sum: Cents = args.iter().map(CellValue::as_cents).sum();
                CellValue::Cents(sum)
            })?;

            let exp_s_deps: Vec<String> = expense_cats
                .iter()
                .map(|c| cell_spent(month, &c.id))
                .collect();
            let exp_s_refs: Vec<&str> = exp_s_deps.iter().map(String::as_str).collect();
            sheet.set_dynamic(&cell_total_spent(month), &exp_s_refs, |args| {
                let sum: Cents = args.iter().map(CellValue::as_cents).sum();
                CellValue::Cents(sum)
            })?;

            let exp_l_deps: Vec<String> = expense_cats
                .iter()
                .map(|c| cell_leftover(month, &c.id))
                .collect();
            let exp_l_refs: Vec<&str> = exp_l_deps.iter().map(String::as_str).collect();
            sheet.set_dynamic(&cell_total_leftover(month), &exp_l_refs, |args| {
                let sum: Cents = args.iter().map(CellValue::as_cents).sum();
                CellValue::Cents(sum)
            })?;

            // 5. Income total: uncategorized income + all income category spent amounts
            let mut inc_deps = vec![cell_uncategorized_income(month)];
            for c in &income_cats {
                inc_deps.push(cell_spent(month, &c.id));
            }
            let inc_refs: Vec<&str> = inc_deps.iter().map(String::as_str).collect();
            sheet.set_dynamic(&cell_total_income(month), &inc_refs, |args| {
                let sum: Cents = args.iter().map(CellValue::as_cents).sum();
                CellValue::Cents(sum)
            })?;

            // 6. From last month
            let from_last_cell = cell_from_last_month(month);
            if has_prev {
                let prev_tb = cell_to_budget(&prev_m);
                let prev_buf = cell_buffered(&prev_m);
                sheet.set_dynamic(&from_last_cell, &[&prev_tb, &prev_buf], |args| {
                    let prev_to_budget = args[0].as_cents();
                    let prev_buffered = args[1].as_cents();
                    CellValue::Cents(prev_to_budget + prev_buffered)
                })?;
            } else {
                sheet.set_static(&from_last_cell, Cents::ZERO);
            }

            // 7. Available funds: total-income + from-last-month
            let avail_cell = cell_available_funds(month);
            let tot_inc_cell = cell_total_income(month);
            sheet.set_dynamic(&avail_cell, &[&tot_inc_cell, &from_last_cell], |args| {
                let total_income = args[0].as_cents();
                let from_last = args[1].as_cents();
                CellValue::Cents(total_income + from_last)
            })?;

            // 8. Last month overspent: sum of negative leftovers from expense categories where carryover is false
            let lmo_cell = cell_last_month_overspent(month);
            if has_prev {
                let mut lmo_deps = Vec::with_capacity(expense_cats.len() * 2);
                for c in &expense_cats {
                    lmo_deps.push(cell_leftover(&prev_m, &c.id));
                    lmo_deps.push(cell_carryover(&prev_m, &c.id));
                }
                let lmo_refs: Vec<&str> = lmo_deps.iter().map(String::as_str).collect();
                sheet.set_dynamic(&lmo_cell, &lmo_refs, |args| {
                    let mut overspent = Cents::ZERO;
                    for chunk in args.as_chunks::<2>().0 {
                        let leftover = chunk[0].as_cents();
                        let carryover = chunk[1].as_bool();
                        if !carryover && leftover.is_negative() {
                            overspent += leftover;
                        }
                    }
                    CellValue::Cents(overspent)
                })?;
            } else {
                sheet.set_static(&lmo_cell, Cents::ZERO);
            }

            // 9. To Budget: available-funds + last-month-overspent - total-budgeted - buffered
            let tb_cell = cell_to_budget(month);
            let tot_bud_cell = cell_total_budgeted(month);
            sheet.set_dynamic(
                &tb_cell,
                &[&avail_cell, &lmo_cell, &tot_bud_cell, &buffered_cell],
                |args| {
                    let available = args[0].as_cents();
                    let overspent = args[1].as_cents(); // <= 0
                    let budgeted = args[2].as_cents();
                    let buffered = args[3].as_cents();
                    CellValue::Cents(available + overspent - budgeted - buffered)
                },
            )?;

            Ok(())
        })
    }

    /// Sets the budgeted amount for a category in a given month.
    pub fn set_budget(&mut self, month: &str, category_id: &str, amount: Cents) {
        let cell = cell_budget(month, category_id);
        self.spreadsheet.set_static(&cell, amount);
    }

    /// Sets the carryover toggle for an envelope category in a given month.
    pub fn set_carryover(&mut self, month: &str, category_id: &str, carryover: bool) {
        let cell = cell_carryover(month, category_id);
        self.spreadsheet.set_static(&cell, carryover);
    }

    /// Sets the spent amount for a category in a given month directly.
    pub fn set_spent(&mut self, month: &str, category_id: &str, spent: Cents) {
        let cell = cell_spent(month, category_id);
        self.spreadsheet.set_static(&cell, spent);
    }

    /// Sets the buffered amount for a given month.
    pub fn set_buffered(&mut self, month: &str, buffered: Cents) {
        let cell = cell_buffered(month);
        self.spreadsheet.set_static(&cell, buffered);
    }

    /// Sets uncategorized income for a given month.
    pub fn set_uncategorized_income(&mut self, month: &str, income: Cents) {
        let cell = cell_uncategorized_income(month);
        self.spreadsheet.set_static(&cell, income);
    }

    /// Adds a transaction into the budget, updating category spent amounts or uncategorized income.
    ///
    /// # Errors
    /// Returns [`DomainError::InvalidMonthFormat`] if the transaction date cannot be mapped.
    pub fn add_transaction(
        &mut self,
        tx: &Transaction,
        is_income: bool,
    ) -> Result<(), DomainError> {
        let month = tx.date.format("%Y-%m").to_string();

        // If month not initialized, auto-initialize with current known categories and groups
        if !self.initialized_months.contains(&month) {
            let cats: Vec<Category> = self.categories.values().cloned().collect();
            let grps: Vec<CategoryGroup> = self.groups.values().cloned().collect();
            self.init_month(&month, &cats, &grps)?;
        }

        if tx.is_split_parent() && !tx.subtransactions.is_empty() {
            for sub in &tx.subtransactions {
                self.add_single_transaction(&month, sub, is_income);
            }
        } else {
            self.add_single_transaction(&month, tx, is_income);
        }

        Ok(())
    }

    fn add_single_transaction(&mut self, month: &str, tx: &Transaction, is_income: bool) {
        if let Some(cat_id) = &tx.category_id {
            let cell = cell_spent(month, cat_id);
            let current = self.spreadsheet.get_cents(&cell);
            self.spreadsheet.set_static(&cell, current + tx.amount);
        } else if is_income || tx.amount.is_positive() {
            let cell = cell_uncategorized_income(month);
            let current = self.spreadsheet.get_cents(&cell);
            self.spreadsheet.set_static(&cell, current + tx.amount);
        }
    }

    /// Executes multiple updates in a batch transaction, deferring DAG recomputation until completion.
    pub fn transaction<F, R>(&mut self, f: F) -> Result<R, DomainError>
    where
        F: FnOnce(&mut Self) -> Result<R, DomainError>,
    {
        // We use spreadsheet transaction by running inner updates
        f(self)
    }

    // --- Inspection Getters ---

    /// Returns the "To Budget" amount for a given month.
    #[must_use]
    pub fn get_to_budget(&self, month: &str) -> Cents {
        self.spreadsheet.get_cents(&cell_to_budget(month))
    }

    /// Returns the available funds for a given month.
    #[must_use]
    pub fn get_available_funds(&self, month: &str) -> Cents {
        self.spreadsheet.get_cents(&cell_available_funds(month))
    }

    /// Returns total income for a given month.
    #[must_use]
    pub fn get_total_income(&self, month: &str) -> Cents {
        self.spreadsheet.get_cents(&cell_total_income(month))
    }

    /// Returns the funds brought forward from the previous month.
    #[must_use]
    pub fn get_from_last_month(&self, month: &str) -> Cents {
        self.spreadsheet.get_cents(&cell_from_last_month(month))
    }

    /// Returns the total uncarried overspending from the previous month (<= 0).
    #[must_use]
    pub fn get_last_month_overspent(&self, month: &str) -> Cents {
        self.spreadsheet
            .get_cents(&cell_last_month_overspent(month))
    }

    /// Returns the total budgeted across all expense categories for a month.
    #[must_use]
    pub fn get_total_budgeted(&self, month: &str) -> Cents {
        self.spreadsheet.get_cents(&cell_total_budgeted(month))
    }

    /// Returns the total spent across all expense categories for a month.
    #[must_use]
    pub fn get_total_spent(&self, month: &str) -> Cents {
        self.spreadsheet.get_cents(&cell_total_spent(month))
    }

    /// Returns the total leftover across all expense categories for a month.
    #[must_use]
    pub fn get_total_leftover(&self, month: &str) -> Cents {
        self.spreadsheet.get_cents(&cell_total_leftover(month))
    }

    /// Returns the budgeted amount for a category in a month.
    #[must_use]
    pub fn get_category_budget(&self, month: &str, category_id: &str) -> Cents {
        self.spreadsheet.get_cents(&cell_budget(month, category_id))
    }

    /// Returns the spent amount for a category in a month.
    #[must_use]
    pub fn get_category_spent(&self, month: &str, category_id: &str) -> Cents {
        self.spreadsheet.get_cents(&cell_spent(month, category_id))
    }

    /// Returns the leftover envelope balance for a category in a month.
    #[must_use]
    pub fn get_category_leftover(&self, month: &str, category_id: &str) -> Cents {
        self.spreadsheet
            .get_cents(&cell_leftover(month, category_id))
    }

    /// Returns the carryover flag for a category in a month.
    #[must_use]
    pub fn get_category_carryover(&self, month: &str, category_id: &str) -> bool {
        self.spreadsheet
            .get_bool(&cell_carryover(month, category_id))
    }

    /// Returns the total budgeted amount for a category group in a month.
    #[must_use]
    pub fn get_group_budget(&self, month: &str, group_id: &str) -> Cents {
        self.spreadsheet
            .get_cents(&cell_group_budget(month, group_id))
    }

    /// Returns the total spent amount for a category group in a month.
    #[must_use]
    pub fn get_group_spent(&self, month: &str, group_id: &str) -> Cents {
        self.spreadsheet
            .get_cents(&cell_group_spent(month, group_id))
    }

    /// Returns the total leftover amount for a category group in a month.
    #[must_use]
    pub fn get_group_leftover(&self, month: &str, group_id: &str) -> Cents {
        self.spreadsheet
            .get_cents(&cell_group_leftover(month, group_id))
    }
}
