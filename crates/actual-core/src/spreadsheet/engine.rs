//! Spreadsheet calculation engine managing cell values, dynamic formulas, and topological evaluation.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::cents::Cents;
use crate::error::DomainError;
use crate::spreadsheet::cell::CellValue;
use crate::spreadsheet::graph::DependencyGraph;

/// Type alias for cell formula evaluation functions.
pub type FormulaFn = Arc<dyn Fn(&[CellValue]) -> CellValue + Send + Sync>;

/// Structure tracking a dynamic formula and its dependency cell names.
#[derive(Clone)]
pub struct DynamicCell {
    pub dependencies: Vec<String>,
    pub formula: FormulaFn,
}

/// The core reactive spreadsheet engine.
#[derive(Default)]
pub struct Spreadsheet {
    /// Dependency DAG tracking cell relationships.
    graph: DependencyGraph,
    /// Current calculated or static values for all cells.
    values: HashMap<String, CellValue>,
    /// Dynamic formula definitions.
    dynamic_cells: HashMap<String, DynamicCell>,
    /// Cells marked as changed/dirty awaiting recomputation.
    dirty_cells: HashSet<String>,
    /// Transaction nesting depth.
    transaction_depth: usize,
}

impl Spreadsheet {
    /// Creates a new empty `Spreadsheet`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets a static cell value directly.
    ///
    /// Clears any dynamic formula previously associated with this cell.
    pub fn set_static(&mut self, name: &str, value: impl Into<CellValue>) {
        let val = value.into();

        // Clear previous incoming edges if this cell was previously dynamic
        if self.dynamic_cells.remove(name).is_some() {
            self.graph.clear_dependencies(name);
        }

        self.graph.get_or_insert_node(name);
        self.values.insert(name.to_string(), val);
        self.dirty_cells.insert(name.to_string());

        if self.transaction_depth == 0 {
            let _ = self.recompute();
        }
    }

    /// Defines a dynamic cell whose value is computed reactively from its dependencies.
    ///
    /// # Errors
    /// Returns [`DomainError::CycleDetected`] if the new formula introduces a cyclic dependency.
    pub fn set_dynamic<F>(
        &mut self,
        name: &str,
        dependencies: &[&str],
        formula: F,
    ) -> Result<(), DomainError>
    where
        F: Fn(&[CellValue]) -> CellValue + Send + Sync + 'static,
    {
        self.graph.clear_dependencies(name);
        self.graph.get_or_insert_node(name);

        for &dep in dependencies {
            self.graph.get_or_insert_node(dep);
            self.graph.add_dependency(dep, name)?;
        }

        let dep_strings: Vec<String> = dependencies.iter().map(|&s| s.to_string()).collect();
        let formula_fn = Arc::new(formula);

        // Evaluate initial value
        let arg_values: Vec<CellValue> = dep_strings
            .iter()
            .map(|dep| self.get_value(dep).cloned().unwrap_or_default())
            .collect();
        let initial_val = formula_fn(&arg_values);

        self.values.insert(name.to_string(), initial_val);
        self.dynamic_cells.insert(
            name.to_string(),
            DynamicCell {
                dependencies: dep_strings,
                formula: formula_fn,
            },
        );

        self.dirty_cells.insert(name.to_string());

        if self.transaction_depth == 0 {
            let _ = self.recompute()?;
        }

        Ok(())
    }

    /// Returns a reference to a cell's value if it exists.
    #[must_use]
    pub fn get_value(&self, name: &str) -> Option<&CellValue> {
        self.values.get(name)
    }

    /// Returns the cell's value as `Cents` (defaults to `Cents::ZERO` if absent).
    #[must_use]
    pub fn get_cents(&self, name: &str) -> Cents {
        self.get_value(name)
            .map(CellValue::as_cents)
            .unwrap_or(Cents::ZERO)
    }

    /// Returns the cell's value as `bool` (defaults to `false` if absent).
    #[must_use]
    pub fn get_bool(&self, name: &str) -> bool {
        self.get_value(name)
            .map(CellValue::as_bool)
            .unwrap_or(false)
    }

    /// Returns the cell's value as `i64` (defaults to `0` if absent).
    #[must_use]
    pub fn get_int(&self, name: &str) -> i64 {
        self.get_value(name).map(CellValue::as_int).unwrap_or(0)
    }

    /// Executes a closure inside a batched transaction, deferring recomputation until the end.
    pub fn transaction<F, R>(&mut self, f: F) -> Result<R, DomainError>
    where
        F: FnOnce(&mut Self) -> Result<R, DomainError>,
    {
        self.transaction_depth += 1;
        let result = f(self);
        self.transaction_depth -= 1;

        if self.transaction_depth == 0 {
            self.recompute()?;
        }

        result
    }

    /// Recomputes all dirty cells and their transitive dependents in topological order.
    ///
    /// Returns the list of cell names that were updated.
    ///
    /// # Errors
    /// Returns [`DomainError::CycleDetected`] if a cycle exists in the dependency chain.
    pub fn recompute(&mut self) -> Result<Vec<String>, DomainError> {
        if self.dirty_cells.is_empty() {
            return Ok(Vec::new());
        }

        let sources: Vec<&str> = self.dirty_cells.iter().map(String::as_str).collect();
        let eval_order = self.graph.topological_sort_dependents(&sources)?;
        self.dirty_cells.clear();

        for cell_name in &eval_order {
            if let Some(dyn_cell) = self.dynamic_cells.get(cell_name).cloned() {
                let args: Vec<CellValue> = dyn_cell
                    .dependencies
                    .iter()
                    .map(|dep| self.get_value(dep).cloned().unwrap_or_default())
                    .collect();

                let new_val = (dyn_cell.formula)(&args);
                self.values.insert(cell_name.clone(), new_val);
            }
        }

        Ok(eval_order)
    }
}
