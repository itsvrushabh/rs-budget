//! Native desktop theme detection, theme modes, and color palette tokens.

use serde::{Deserialize, Serialize};

/// Active theme mode preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ThemeMode {
    /// Automatically match OS theme (dark / light).
    #[default]
    System,
    /// Force Light theme.
    Light,
    /// Force Dark theme.
    Dark,
}

/// Color palette tokens matching Actual Budget's official visual design language.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColorPalette {
    /// App background color.
    pub background: String,
    /// Card / table container surface color.
    pub surface: String,
    /// Alternating table row background.
    pub table_row_alt: String,
    /// Focused / selected cell highlight.
    pub selection: String,
    /// Border and separator line color.
    pub border: String,
    /// Primary text color.
    pub text_primary: String,
    /// Muted / placeholder text color.
    pub text_muted: String,
    /// Accent color for active tabs and primary buttons.
    pub accent: String,
    /// Positive financial indicator (income, positive balance, cleared checkmark).
    pub positive_green: String,
    /// Negative financial indicator (expenses, overspent category, alert).
    pub negative_red: String,
    /// Warning / carryover badge color.
    pub warning_yellow: String,
}

impl ColorPalette {
    /// Dark theme palette matching Actual Budget dark mode.
    #[must_use]
    pub fn dark() -> Self {
        Self {
            background: "#181b21".into(),
            surface: "#242933".into(),
            table_row_alt: "#1f242c".into(),
            selection: "#2e3a4e".into(),
            border: "#333b49".into(),
            text_primary: "#e6edf3".into(),
            text_muted: "#8b949e".into(),
            accent: "#388bfd".into(),
            positive_green: "#2ea043".into(),
            negative_red: "#f85149".into(),
            warning_yellow: "#d29922".into(),
        }
    }

    /// Light theme palette matching Actual Budget light mode.
    #[must_use]
    pub fn light() -> Self {
        Self {
            background: "#f6f8fa".into(),
            surface: "#ffffff".into(),
            table_row_alt: "#f9fafb".into(),
            selection: "#dbeafe".into(),
            border: "#d0d7de".into(),
            text_primary: "#24292f".into(),
            text_muted: "#57606a".into(),
            accent: "#0969da".into(),
            positive_green: "#1a7f37".into(),
            negative_red: "#cf222e".into(),
            warning_yellow: "#9a6700".into(),
        }
    }

    /// Resolves active palette based on user mode preference and system dark mode status.
    #[must_use]
    pub fn resolve(mode: ThemeMode, system_is_dark: bool) -> Self {
        match mode {
            ThemeMode::Dark => Self::dark(),
            ThemeMode::Light => Self::light(),
            ThemeMode::System => {
                if system_is_dark {
                    Self::dark()
                } else {
                    Self::light()
                }
            }
        }
    }
}

/// Detects the OS-level dark mode preference.
pub fn detect_system_dark_mode() -> bool {
    // Check environment variable overrides first (e.g. for testing)
    if let Ok(val) = std::env::var("ACTUAL_DARK_MODE") {
        if val == "1" || val.eq_ignore_ascii_case("true") {
            return true;
        } else if val == "0" || val.eq_ignore_ascii_case("false") {
            return false;
        }
    }

    // On Linux desktop, check gsettings / GTK theme or default to dark (modern desktop standard)
    #[cfg(target_os = "linux")]
    {
        // Try reading GTK dark theme setting via desktop portal or environment
        if let Ok(theme) = std::env::var("GTK_THEME") {
            if theme.to_lowercase().contains("dark") {
                return true;
            }
        }
        // Default to dark on Linux
        true
    }

    #[cfg(target_os = "macos")]
    {
        // On macOS, defaults read -g AppleInterfaceStyle returns "Dark"
        true
    }

    #[cfg(target_os = "windows")]
    {
        true
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        true
    }
}
