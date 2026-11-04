//! Keyboard navigation state machine, cell cycling, and IME input buffering.

use crate::error::GuiError;
use crate::table::EditableTable;
use unicode_segmentation::UnicodeSegmentation;

/// Logical keyboard navigation action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyAction {
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,
    MoveFirst,
    MoveLast,
    PageUp,
    PageDown,
    NextCell,   // Tab
    PrevCell,   // Shift + Tab
    StartEdit,  // Enter or F2
    CommitEdit, // Enter while editing
    CancelEdit, // Escape
}

/// Keyboard navigation controller managing focus movement and editing transitions.
#[derive(Debug, Clone, Default)]
pub struct KeyboardNavigator;

impl KeyboardNavigator {
    /// Applies a navigation action to an editable table.
    pub fn handle_action(table: &mut EditableTable, action: KeyAction) -> Result<bool, GuiError> {
        let row_count = table.row_count();
        let col_count = table.col_count();

        if row_count == 0 || col_count == 0 {
            return Ok(false);
        }

        let curr_row = table.selection.row;
        let curr_col = table.selection.col;
        let is_editing = table.get_cell(curr_row, curr_col)?.is_editing();

        match action {
            KeyAction::StartEdit => {
                if !is_editing {
                    table.start_editing_selected()?;
                    return Ok(true);
                }
            }
            KeyAction::CommitEdit => {
                if is_editing {
                    table.commit_editing_selected()?;
                    // Advance to next row on enter (spreadsheet behavior)
                    if curr_row + 1 < row_count {
                        table.set_selection(curr_row + 1, curr_col);
                    }
                    return Ok(true);
                }
            }
            KeyAction::CancelEdit => {
                if is_editing {
                    table.cancel_editing_selected();
                    return Ok(true);
                }
            }
            KeyAction::MoveUp => {
                if !is_editing && curr_row > 0 {
                    table.set_selection(curr_row - 1, curr_col);
                    return Ok(true);
                }
            }
            KeyAction::MoveDown => {
                if !is_editing && curr_row + 1 < row_count {
                    table.set_selection(curr_row + 1, curr_col);
                    return Ok(true);
                }
            }
            KeyAction::MoveLeft => {
                if !is_editing && curr_col > 0 {
                    table.set_selection(curr_row, curr_col - 1);
                    return Ok(true);
                }
            }
            KeyAction::MoveRight => {
                if !is_editing && curr_col + 1 < col_count {
                    table.set_selection(curr_row, curr_col + 1);
                    return Ok(true);
                }
            }
            KeyAction::MoveFirst => {
                if !is_editing {
                    table.set_selection(0, curr_col);
                    return Ok(true);
                }
            }
            KeyAction::MoveLast => {
                if !is_editing {
                    table.set_selection(row_count - 1, curr_col);
                    return Ok(true);
                }
            }
            KeyAction::PageUp => {
                if !is_editing {
                    let page_size = (table.virtual_scroll.viewport_height
                        / table.virtual_scroll.row_height)
                        .max(1) as usize;
                    table.set_selection(curr_row.saturating_sub(page_size), curr_col);
                    return Ok(true);
                }
            }
            KeyAction::PageDown => {
                if !is_editing {
                    let page_size = (table.virtual_scroll.viewport_height
                        / table.virtual_scroll.row_height)
                        .max(1) as usize;
                    table.set_selection((curr_row + page_size).min(row_count - 1), curr_col);
                    return Ok(true);
                }
            }
            KeyAction::NextCell => {
                // If editing, commit first
                if is_editing {
                    table.commit_editing_selected()?;
                }
                // Cycle right, wrapping to next row
                if curr_col + 1 < col_count {
                    table.set_selection(curr_row, curr_col + 1);
                } else if curr_row + 1 < row_count {
                    table.set_selection(curr_row + 1, 0);
                } else {
                    table.set_selection(0, 0);
                }
                return Ok(true);
            }
            KeyAction::PrevCell => {
                // If editing, commit first
                if is_editing {
                    table.commit_editing_selected()?;
                }
                // Cycle left, wrapping to previous row
                if curr_col > 0 {
                    table.set_selection(curr_row, curr_col - 1);
                } else if curr_row > 0 {
                    table.set_selection(curr_row - 1, col_count - 1);
                } else {
                    table.set_selection(row_count - 1, col_count - 1);
                }
                return Ok(true);
            }
        }

        Ok(false)
    }
}

/// Input Method Editor (IME) composition buffer supporting CJK and accented text input.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImeBuffer {
    /// Active pre-edit / composition string.
    pub preedit: String,
    /// Cursor position within pre-edit string (grapheme cluster offset).
    pub cursor: usize,
    /// Whether IME composition is currently active.
    pub is_composing: bool,
    /// List of candidate strings provided by the IME engine.
    pub candidates: Vec<String>,
    /// Selected candidate index.
    pub selected_candidate: Option<usize>,
}

impl ImeBuffer {
    /// Creates a new empty IME buffer.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Starts or updates IME composition with a pre-edit string and cursor index.
    pub fn update_preedit(&mut self, text: impl Into<String>, cursor: usize) {
        self.preedit = text.into();
        self.is_composing = !self.preedit.is_empty();
        let grapheme_count = self.preedit.graphemes(true).count();
        self.cursor = cursor.min(grapheme_count);
    }

    /// Sets active candidate list.
    pub fn set_candidates(&mut self, candidates: Vec<String>) {
        self.candidates = candidates;
        self.selected_candidate = if self.candidates.is_empty() {
            None
        } else {
            Some(0)
        };
    }

    /// Selects the next candidate.
    pub fn next_candidate(&mut self) {
        if let Some(curr) = self.selected_candidate {
            if !self.candidates.is_empty() {
                self.selected_candidate = Some((curr + 1) % self.candidates.len());
            }
        }
    }

    /// Selects the previous candidate.
    pub fn prev_candidate(&mut self) {
        if let Some(curr) = self.selected_candidate {
            if !self.candidates.is_empty() {
                self.selected_candidate = Some(if curr == 0 {
                    self.candidates.len() - 1
                } else {
                    curr - 1
                });
            }
        }
    }

    /// Commits the active composition (or selected candidate), returning final text and clearing buffer.
    pub fn commit(&mut self) -> String {
        let result = if let Some(idx) = self.selected_candidate {
            if let Some(cand) = self.candidates.get(idx) {
                cand.clone()
            } else {
                std::mem::take(&mut self.preedit)
            }
        } else {
            std::mem::take(&mut self.preedit)
        };

        self.reset();
        result
    }

    /// Cancels active composition, resetting buffer.
    pub fn reset(&mut self) {
        self.preedit.clear();
        self.cursor = 0;
        self.is_composing = false;
        self.candidates.clear();
        self.selected_candidate = None;
    }
}
