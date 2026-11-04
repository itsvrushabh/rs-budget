//! `actual-import` parses financial statement files (QIF, OFX, CSV) and provides
//! payee normalization and transaction deduplication for rs-budget.

pub mod cleaner;
pub mod csv_parser;
pub mod dedup;
pub mod error;
pub mod models;
pub mod ofx;
pub mod qif;

pub use cleaner::PayeeCleaner;
pub use csv_parser::{CsvConfig, CsvParser};
pub use dedup::{DedupResult, Deduplicator};
pub use error::ImportError;
pub use models::{ImportedSplit, ImportedTransaction};
pub use ofx::OfxParser;
pub use qif::QifParser;
