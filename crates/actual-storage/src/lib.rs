//! `actual-storage` handles SQLite database connections, schema migrations,
//! and native Actual Budget file reading.

/// Trait defining storage operations for budgets.
pub trait BudgetStore {
    // Methods will be added in Phase 1
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(1 + 1, 2);
    }
}
