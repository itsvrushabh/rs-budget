//! `actual-core` defines domain models, monetary precision,
//! envelope budgeting calculations, and deterministic transaction rules.

pub mod models;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
