//! Transaction deduplication and proximity matching.
//!
//! Prevents duplicate imports using strict external ID matching (`imported_id` / `FITID`)
//! and fuzzy proximity matching (matching amount within a date window).

use crate::models::ImportedTransaction;
use actual_core::Transaction;
use std::collections::HashSet;

/// Result of a deduplication process categorizing imported items into unique vs duplicate.
#[derive(Debug, Clone)]
pub struct DedupResult {
    /// Transactions identified as unique and safe to insert.
    pub unique: Vec<ImportedTransaction>,
    /// Transactions identified as duplicate, alongside the ID of the matching existing transaction.
    pub duplicates: Vec<(ImportedTransaction, String)>,
}

/// Deduplication engine coordinating strict and proximity matching against existing transactions.
#[derive(Debug, Clone, Default)]
pub struct Deduplicator {
    date_proximity_days: i64,
}

impl Deduplicator {
    /// Constructs a `Deduplicator` with a specified proximity day window (default 3 days).
    #[must_use]
    pub fn new(date_proximity_days: i64) -> Self {
        Self {
            date_proximity_days: date_proximity_days.max(0),
        }
    }

    /// Deduplicates a batch of imported transactions against existing transactions in the account.
    ///
    /// Deduplication rules:
    /// 1. **Strict Match**: If incoming transaction has an `imported_id` matching an active existing
    ///    transaction's `imported_id` in the same account, it is flagged as duplicate.
    /// 2. **Proximity Match**: If incoming transaction matches an existing active transaction with:
    ///    - Exact same amount (`tx.amount == incoming.amount`)
    ///    - Date difference `<= date_proximity_days` (e.g. +/- 3 days)
    ///    - The existing transaction has not already been matched during this run.
    #[must_use]
    pub fn deduplicate(
        &self,
        incoming: Vec<ImportedTransaction>,
        existing: &[Transaction],
    ) -> DedupResult {
        let mut unique = Vec::new();
        let mut duplicates = Vec::new();

        // Track which existing transaction IDs have been matched to prevent 1-to-many matches.
        let mut matched_existing_ids: HashSet<String> = HashSet::new();

        for item in incoming {
            let mut found_duplicate = None;

            // 1. Strict match on imported_id
            if let Some(ref imp_id) = item.imported_id {
                for ex in existing {
                    if ex.tombstone {
                        continue;
                    }
                    if let Some(ref ex_imp_id) = ex.imported_id {
                        if ex_imp_id == imp_id {
                            found_duplicate = Some(ex.id.clone());
                            break;
                        }
                    }
                }
            }

            // 2. Proximity fallback
            if found_duplicate.is_none() && self.date_proximity_days > 0 {
                for ex in existing {
                    if ex.tombstone || matched_existing_ids.contains(&ex.id) {
                        continue;
                    }

                    if ex.amount == item.amount {
                        let diff_days = (ex.date - item.date).num_days().abs();
                        if diff_days <= self.date_proximity_days {
                            found_duplicate = Some(ex.id.clone());
                            break;
                        }
                    }
                }
            }

            if let Some(matched_id) = found_duplicate {
                matched_existing_ids.insert(matched_id.clone());
                duplicates.push((item, matched_id));
            } else {
                unique.push(item);
            }
        }

        DedupResult { unique, duplicates }
    }
}
