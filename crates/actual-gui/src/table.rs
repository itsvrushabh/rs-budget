//! Editable financial table components, cell models, virtualization, and selection.

use crate::error::GuiError;
use actual_core::cents::Cents;
use serde::{Deserialize, Serialize};

/// Data type and value stored within a table cell.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CellValue {
    /// Text string (e.g. category name, payee, notes).
    Text(String),
    /// Monetary cents amount.
    Currency(Cents),
    /// ISO-8601 date string (e.g. "2024-05-15").
    Date(String),
    /// Boolean flag (e.g. cleared status, carryover enabled).
    Boolean(bool),
    /// Empty or null cell.
    Empty,
}

impl CellValue {
    /// Formats the cell value for human display.
    #[must_use]
    pub fn display_string(&self) -> String {
        match self {
            Self::Text(s) => s.clone(),
            Self::Currency(c) => c.to_decimal_string(2),
            Self::Date(d) => d.clone(),
            Self::Boolean(b) => {
                if *b {
                    "✓".into()
                } else {
                    "".into()
                }
            }
            Self::Empty => String::new(),
        }
    }

    /// Attempts to parse raw user text input into this cell's expected type.
    pub fn parse_input(&self, input: &str) -> Result<Self, String> {
        let trimmed = input.trim();
        match self {
            Self::Currency(_) | Self::Empty => {
                if trimmed.is_empty() {
                    Ok(Self::Currency(Cents::ZERO))
                } else {
                    trimmed
                        .parse::<Cents>()
                        .map(Self::Currency)
                        .map_err(|e| format!("Invalid currency format: {e}"))
                }
            }
            Self::Text(_) => Ok(Self::Text(trimmed.to_string())),
            Self::Date(_) => {
                if trimmed.len() == 10
                    && trimmed.chars().nth(4) == Some('-')
                    && trimmed.chars().nth(7) == Some('-')
                {
                    Ok(Self::Date(trimmed.to_string()))
                } else {
                    Err("Expected YYYY-MM-DD date format".into())
                }
            }
            Self::Boolean(_) => match trimmed.to_lowercase().as_str() {
                "true" | "1" | "yes" | "✓" => Ok(Self::Boolean(true)),
                "false" | "0" | "no" | "" => Ok(Self::Boolean(false)),
                _ => Err("Expected true/false boolean".into()),
            },
        }
    }
}

/// Horizontal alignment of cell content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColumnAlign {
    Left,
    Center,
    Right,
}

/// Definition of a table column.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableColumn {
    /// Unique column identifier (e.g. "name", "budgeted", "activity", "balance").
    pub id: String,
    /// Header display title (e.g. "Category", "Budgeted", "Activity", "Balance").
    pub title: String,
    /// Column width in pixels or layout units.
    pub width: u32,
    /// Text alignment.
    pub align: ColumnAlign,
    /// Whether the user can edit cells in this column inline.
    pub is_editable: bool,
    /// Whether the column is sortable.
    pub is_sortable: bool,
}

impl TableColumn {
    pub fn new(id: impl Into<String>, title: impl Into<String>, width: u32) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            width,
            align: ColumnAlign::Left,
            is_editable: false,
            is_sortable: true,
        }
    }

    pub fn with_align(mut self, align: ColumnAlign) -> Self {
        self.align = align;
        self
    }

    pub fn editable(mut self, editable: bool) -> Self {
        self.is_editable = editable;
        self
    }
}

/// A single cell within the editable table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableCell {
    /// Current committed value.
    pub value: CellValue,
    /// Buffer when actively editing cell.
    pub edit_buffer: Option<String>,
    /// Whether this cell has unsaved modifications.
    pub is_dirty: bool,
    /// Tooltip or validation error message if invalid.
    pub error: Option<String>,
}

impl TableCell {
    pub fn new(value: CellValue) -> Self {
        Self {
            value,
            edit_buffer: None,
            is_dirty: false,
            error: None,
        }
    }

    /// Whether the cell is currently in active inline edit mode.
    #[must_use]
    pub fn is_editing(&self) -> bool {
        self.edit_buffer.is_some()
    }

    /// Begins editing this cell, pre-populating the edit buffer.
    pub fn start_editing(&mut self) {
        self.edit_buffer = Some(self.value.display_string());
        self.error = None;
    }

    /// Cancels active editing, discarding buffer.
    pub fn cancel_editing(&mut self) {
        self.edit_buffer = None;
        self.error = None;
    }

    /// Commits the active edit buffer into cell value.
    pub fn commit_editing(&mut self) -> Result<CellValue, String> {
        if let Some(buf) = self.edit_buffer.take() {
            match self.value.parse_input(&buf) {
                Ok(new_val) => {
                    if new_val != self.value {
                        self.value = new_val.clone();
                        self.is_dirty = true;
                    }
                    self.error = None;
                    Ok(new_val)
                }
                Err(err) => {
                    self.error = Some(err.clone());
                    self.edit_buffer = Some(buf);
                    Err(err)
                }
            }
        } else {
            Ok(self.value.clone())
        }
    }
}

/// Tracks selected row and column coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SelectionModel {
    /// Currently focused row index.
    pub row: usize,
    /// Currently focused column index.
    pub col: usize,
}

/// Virtualized viewport configuration for high-performance rendering of 10,000+ financial rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtualScroll {
    /// Total number of rows in dataset.
    pub total_rows: usize,
    /// Height of each row in pixels.
    pub row_height: u32,
    /// Visible container height in pixels.
    pub viewport_height: u32,
    /// Vertical scroll offset in pixels.
    pub scroll_top: u32,
    /// Overscan buffer rows above and below visible area.
    pub overscan: usize,
}

impl VirtualScroll {
    pub fn new(total_rows: usize, row_height: u32, viewport_height: u32) -> Self {
        Self {
            total_rows,
            row_height: row_height.max(1),
            viewport_height,
            scroll_top: 0,
            overscan: 3,
        }
    }

    /// Computes the start (inclusive) and end (exclusive) row indices that need to be rendered.
    #[must_use]
    pub fn visible_range(&self) -> (usize, usize) {
        if self.total_rows == 0 {
            return (0, 0);
        }

        let first_visible = (self.scroll_top / self.row_height) as usize;
        let visible_count = self.viewport_height.div_ceil(self.row_height) as usize;

        let start = first_visible.saturating_sub(self.overscan);
        let end = (first_visible + visible_count + self.overscan).min(self.total_rows);

        (start, end)
    }

    /// Scrolls to ensure a given row index is visible in the viewport.
    pub fn scroll_to_row(&mut self, row: usize) {
        let row_top = (row as u32) * self.row_height;
        let row_bottom = row_top + self.row_height;

        if row_top < self.scroll_top {
            self.scroll_top = row_top;
        } else if row_bottom > self.scroll_top + self.viewport_height {
            self.scroll_top = row_bottom.saturating_sub(self.viewport_height);
        }
    }
}

/// Generic, high-performance editable table model for financial ledgers.
#[derive(Debug, Clone)]
pub struct EditableTable {
    /// Column specifications.
    pub columns: Vec<TableColumn>,
    /// 2D grid of rows and cells: `cells[row][col]`.
    pub rows: Vec<Vec<TableCell>>,
    /// Current cell selection.
    pub selection: SelectionModel,
    /// Virtual scroll manager.
    pub virtual_scroll: VirtualScroll,
    /// Optional entity ID associated with each row (e.g. category ID, transaction ID).
    pub row_ids: Vec<String>,
}

impl EditableTable {
    /// Creates a new editable table with given column schemas and viewport height.
    pub fn new(columns: Vec<TableColumn>, row_height: u32, viewport_height: u32) -> Self {
        Self {
            columns,
            rows: Vec::new(),
            selection: SelectionModel::default(),
            virtual_scroll: VirtualScroll::new(0, row_height, viewport_height),
            row_ids: Vec::new(),
        }
    }

    /// Adds a row to the table with its corresponding entity ID.
    pub fn add_row(&mut self, row_id: impl Into<String>, cells: Vec<TableCell>) {
        assert_eq!(
            cells.len(),
            self.columns.len(),
            "Cell count must match column count"
        );
        self.rows.push(cells);
        self.row_ids.push(row_id.into());
        self.virtual_scroll.total_rows = self.rows.len();
    }

    /// Clears all rows from the table.
    pub fn clear(&mut self) {
        self.rows.clear();
        self.row_ids.clear();
        self.selection = SelectionModel::default();
        self.virtual_scroll.total_rows = 0;
        self.virtual_scroll.scroll_top = 0;
    }

    /// Row count.
    #[must_use]
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// Column count.
    #[must_use]
    pub fn col_count(&self) -> usize {
        self.columns.len()
    }

    /// Accesses a cell by row and col index.
    pub fn get_cell(&self, row: usize, col: usize) -> Result<&TableCell, GuiError> {
        self.rows
            .get(row)
            .and_then(|r| r.get(col))
            .ok_or(GuiError::OutOfBounds(row, col))
    }

    /// Mutably accesses a cell by row and col index.
    pub fn get_cell_mut(&mut self, row: usize, col: usize) -> Result<&mut TableCell, GuiError> {
        self.rows
            .get_mut(row)
            .and_then(|r| r.get_mut(col))
            .ok_or(GuiError::OutOfBounds(row, col))
    }

    /// Begins inline editing of the currently selected cell if editable.
    pub fn start_editing_selected(&mut self) -> Result<(), GuiError> {
        let (r, c) = (self.selection.row, self.selection.col);
        if r >= self.rows.len() || c >= self.columns.len() {
            return Err(GuiError::OutOfBounds(r, c));
        }

        if !self.columns[c].is_editable {
            return Err(GuiError::ReadOnlyColumn(self.columns[c].title.clone()));
        }

        self.rows[r][c].start_editing();
        Ok(())
    }

    /// Commits active edit on selected cell.
    pub fn commit_editing_selected(&mut self) -> Result<CellValue, GuiError> {
        let (r, c) = (self.selection.row, self.selection.col);
        if r >= self.rows.len() || c >= self.columns.len() {
            return Err(GuiError::OutOfBounds(r, c));
        }

        self.rows[r][c]
            .commit_editing()
            .map_err(|reason| GuiError::ValidationFailed {
                row: r,
                col: c,
                reason,
            })
    }

    /// Cancels active edit on selected cell.
    pub fn cancel_editing_selected(&mut self) {
        let (r, c) = (self.selection.row, self.selection.col);
        if r < self.rows.len() && c < self.columns.len() {
            self.rows[r][c].cancel_editing();
        }
    }

    /// Sets selection to row and col, ensuring visible in viewport.
    pub fn set_selection(&mut self, row: usize, col: usize) {
        if self.rows.is_empty() || self.columns.is_empty() {
            self.selection = SelectionModel::default();
            return;
        }

        self.selection.row = row.min(self.rows.len() - 1);
        self.selection.col = col.min(self.columns.len() - 1);
        self.virtual_scroll.scroll_to_row(self.selection.row);
    }
}
