//! Spending breakdown by category and category group.
//!
//! Analyzes expense transactions to compute total amounts spent per category
//! and group, along with relative percentage shares.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::category::{Category, CategoryGroup};
use crate::cents::Cents;
use crate::reports::filters::ReportFilter;
use crate::transaction::Transaction;

/// Spending summary for a specific category.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CategorySpending {
    /// Category unique identifier.
    pub category_id: String,
    /// Category name.
    pub category_name: String,
    /// Total outflow spent in this category (positive cents).
    pub amount: Cents,
    /// Percentage of total spending across all categories in the report (0.0 to 100.0).
    pub percentage: f64,
}

/// Spending summary for a category group.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GroupSpending {
    /// Group unique identifier.
    pub group_id: String,
    /// Group name.
    pub group_name: String,
    /// Total outflow spent across all categories in this group.
    pub amount: Cents,
    /// Percentage of total spending across all groups in the report (0.0 to 100.0).
    pub percentage: f64,
    /// Breakdown of individual categories within this group.
    pub categories: Vec<CategorySpending>,
}

/// Computed spending breakdown report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpendingReport {
    /// Total expenditure across all filtered categories.
    pub total_spending: Cents,
    /// Group-level spending breakdown sorted by amount descending.
    pub groups: Vec<GroupSpending>,
    /// Flat list of category spending sorted by amount descending.
    pub uncategorized_spending: Cents,
}

impl SpendingReport {
    /// Computes spending breakdown for categories and category groups.
    ///
    /// Only negative transaction amounts (expenses/outflows) are accumulated.
    /// Inflows and transfers are excluded.
    #[must_use]
    pub fn compute(
        transactions: &[Transaction],
        categories: &[Category],
        groups: &[CategoryGroup],
        filter: &ReportFilter,
    ) -> Self {
        let cat_map: HashMap<&str, &Category> =
            categories.iter().map(|c| (c.id.as_str(), c)).collect();
        let grp_map: HashMap<&str, &CategoryGroup> =
            groups.iter().map(|g| (g.id.as_str(), g)).collect();

        let mut cat_totals: HashMap<String, i64> = HashMap::new();
        let mut uncategorized_total = 0i64;
        let mut total_cents = 0i64;

        for tx in transactions {
            if tx.tombstone || tx.is_parent || tx.transfer_id.is_some() {
                continue;
            }
            if !filter.matches(tx) {
                continue;
            }

            // Only expenses
            let amount = tx.amount.as_cents();
            if amount < 0 {
                let expense = -amount;
                total_cents += expense;

                if let Some(ref cid) = tx.category_id {
                    *cat_totals.entry(cid.clone()).or_insert(0) += expense;
                } else {
                    uncategorized_total += expense;
                }
            }
        }

        let total_f64 = if total_cents > 0 {
            total_cents as f64
        } else {
            1.0
        };

        // Group categories by group_id
        let mut group_cats: HashMap<String, Vec<CategorySpending>> = HashMap::new();

        for (cid, cents) in &cat_totals {
            let cat_name = cat_map
                .get(cid.as_str())
                .map(|c| c.name.clone())
                .unwrap_or_else(|| cid.clone());
            let group_id = cat_map
                .get(cid.as_str())
                .map(|c| c.group_id.clone())
                .unwrap_or_else(|| "ungrouped".to_string());

            let pct = (*cents as f64 / total_f64) * 100.0;
            let cat_spending = CategorySpending {
                category_id: cid.clone(),
                category_name: cat_name,
                amount: Cents::from_cents(*cents),
                percentage: pct,
            };

            group_cats.entry(group_id).or_default().push(cat_spending);
        }

        // Sort categories within each group descending by amount
        for cats in group_cats.values_mut() {
            cats.sort_by_key(|c| std::cmp::Reverse(c.amount));
        }

        let mut groups_report = Vec::new();
        for (gid, mut cats) in group_cats {
            let grp_name = grp_map
                .get(gid.as_str())
                .map(|g| g.name.clone())
                .unwrap_or_else(|| gid.clone());

            let grp_total: i64 = cats.iter().map(|c| c.amount.as_cents()).sum();
            let grp_pct = (grp_total as f64 / total_f64) * 100.0;

            cats.sort_by_key(|c| std::cmp::Reverse(c.amount));

            groups_report.push(GroupSpending {
                group_id: gid,
                group_name: grp_name,
                amount: Cents::from_cents(grp_total),
                percentage: grp_pct,
                categories: cats,
            });
        }

        // Sort groups by amount descending
        groups_report.sort_by_key(|g| std::cmp::Reverse(g.amount));

        Self {
            total_spending: Cents::from_cents(total_cents),
            groups: groups_report,
            uncategorized_spending: Cents::from_cents(uncategorized_total),
        }
    }
}
