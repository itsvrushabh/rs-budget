//! Desktop menu bar specifications, categories, and keyboard shortcut dispatch.

use serde::{Deserialize, Serialize};

/// Keyboard shortcut modifier keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShortcutModifier {
    pub ctrl_or_cmd: bool,
    pub alt: bool,
    pub shift: bool,
}

impl ShortcutModifier {
    pub const NONE: Self = Self {
        ctrl_or_cmd: false,
        alt: false,
        shift: false,
    };
    pub const CTRL: Self = Self {
        ctrl_or_cmd: true,
        alt: false,
        shift: false,
    };
    pub const CTRL_SHIFT: Self = Self {
        ctrl_or_cmd: true,
        alt: false,
        shift: true,
    };
    pub const ALT: Self = Self {
        ctrl_or_cmd: false,
        alt: true,
        shift: false,
    };
}

/// A desktop keyboard shortcut definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Shortcut {
    pub modifier: ShortcutModifier,
    pub key: String,
}

impl Shortcut {
    pub fn new(modifier: ShortcutModifier, key: impl Into<String>) -> Self {
        Self {
            modifier,
            key: key.into(),
        }
    }

    /// Formats the shortcut for display in menus (e.g. "Ctrl+Z" or "Cmd+Z").
    #[must_use]
    pub fn display_label(&self) -> String {
        let mut parts = Vec::new();
        if self.modifier.ctrl_or_cmd {
            #[cfg(target_os = "macos")]
            parts.push("⌘");
            #[cfg(not(target_os = "macos"))]
            parts.push("Ctrl");
        }
        if self.modifier.alt {
            parts.push("Alt");
        }
        if self.modifier.shift {
            parts.push("Shift");
        }
        parts.push(&self.key);
        parts.join("+")
    }
}

/// Identifiers for logical menu actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MenuAction {
    // File
    NewBudget,
    OpenBudget,
    ExportActualArchive,
    ExportCsv,
    Exit,

    // Edit
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    FindTransaction,

    // View
    ToggleTheme,
    ToggleFullscreen,
    ZoomIn,
    ZoomOut,
    ResetZoom,

    // Sync
    SyncNow,
    ResetSync,
    ServerSettings,

    // Help
    Documentation,
    KeyboardShortcuts,
    About,
}

/// A single item within a dropdown menu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MenuItem {
    pub action: MenuAction,
    pub label: String,
    pub shortcut: Option<Shortcut>,
    pub is_enabled: bool,
}

impl MenuItem {
    pub fn new(action: MenuAction, label: impl Into<String>) -> Self {
        Self {
            action,
            label: label.into(),
            shortcut: None,
            is_enabled: true,
        }
    }

    pub fn with_shortcut(mut self, shortcut: Shortcut) -> Self {
        self.shortcut = Some(shortcut);
        self
    }
}

/// Top-level menu categories in the desktop application window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MenuCategory {
    pub title: String,
    pub items: Vec<MenuItem>,
}

/// Desktop menu bar aggregating all categories.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MenuBar {
    pub categories: Vec<MenuCategory>,
}

impl Default for MenuBar {
    fn default() -> Self {
        Self::standard_menu()
    }
}

impl MenuBar {
    /// Constructs the standard Actual Budget desktop menu bar.
    #[must_use]
    pub fn standard_menu() -> Self {
        Self {
            categories: vec![
                MenuCategory {
                    title: "File".into(),
                    items: vec![
                        MenuItem::new(MenuAction::NewBudget, "New Budget...")
                            .with_shortcut(Shortcut::new(ShortcutModifier::CTRL, "N")),
                        MenuItem::new(MenuAction::OpenBudget, "Open Budget...")
                            .with_shortcut(Shortcut::new(ShortcutModifier::CTRL, "O")),
                        MenuItem::new(MenuAction::ExportActualArchive, "Export .actual Archive...")
                            .with_shortcut(Shortcut::new(ShortcutModifier::CTRL_SHIFT, "E")),
                        MenuItem::new(MenuAction::ExportCsv, "Export to CSV..."),
                        MenuItem::new(MenuAction::Exit, "Exit")
                            .with_shortcut(Shortcut::new(ShortcutModifier::CTRL, "Q")),
                    ],
                },
                MenuCategory {
                    title: "Edit".into(),
                    items: vec![
                        MenuItem::new(MenuAction::Undo, "Undo")
                            .with_shortcut(Shortcut::new(ShortcutModifier::CTRL, "Z")),
                        MenuItem::new(MenuAction::Redo, "Redo")
                            .with_shortcut(Shortcut::new(ShortcutModifier::CTRL_SHIFT, "Z")),
                        MenuItem::new(MenuAction::FindTransaction, "Find in Ledger...")
                            .with_shortcut(Shortcut::new(ShortcutModifier::CTRL, "F")),
                    ],
                },
                MenuCategory {
                    title: "View".into(),
                    items: vec![
                        MenuItem::new(MenuAction::ToggleTheme, "Toggle Dark / Light Theme")
                            .with_shortcut(Shortcut::new(ShortcutModifier::CTRL, "T")),
                        MenuItem::new(MenuAction::ToggleFullscreen, "Toggle Fullscreen")
                            .with_shortcut(Shortcut::new(ShortcutModifier::NONE, "F11")),
                        MenuItem::new(MenuAction::ZoomIn, "Zoom In")
                            .with_shortcut(Shortcut::new(ShortcutModifier::CTRL, "+")),
                        MenuItem::new(MenuAction::ZoomOut, "Zoom Out")
                            .with_shortcut(Shortcut::new(ShortcutModifier::CTRL, "-")),
                        MenuItem::new(MenuAction::ResetZoom, "Reset Zoom")
                            .with_shortcut(Shortcut::new(ShortcutModifier::CTRL, "0")),
                    ],
                },
                MenuCategory {
                    title: "Sync".into(),
                    items: vec![
                        MenuItem::new(MenuAction::SyncNow, "Sync Now")
                            .with_shortcut(Shortcut::new(ShortcutModifier::CTRL, "S")),
                        MenuItem::new(MenuAction::ResetSync, "Reset Sync Group..."),
                        MenuItem::new(MenuAction::ServerSettings, "Server Connection Settings..."),
                    ],
                },
                MenuCategory {
                    title: "Help".into(),
                    items: vec![
                        MenuItem::new(MenuAction::Documentation, "Documentation")
                            .with_shortcut(Shortcut::new(ShortcutModifier::NONE, "F1")),
                        MenuItem::new(MenuAction::KeyboardShortcuts, "Keyboard Shortcuts..."),
                        MenuItem::new(MenuAction::About, "About Actual Budget"),
                    ],
                },
            ],
        }
    }
}
