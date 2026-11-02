//! Reactive spreadsheet calculation engine backed by a directed acyclic graph (DAG).

pub mod cell;
pub mod engine;
pub mod graph;

pub use cell::CellValue;
pub use engine::{DynamicCell, FormulaFn, Spreadsheet};
pub use graph::{DependencyGraph, NodeData, NodeKey};
