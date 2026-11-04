//! Desktop window state, dimensions, and configuration persistence.

use serde::{Deserialize, Serialize};

/// Window dimensions and position on the desktop display.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowDimensions {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Default for WindowDimensions {
    fn default() -> Self {
        Self {
            x: 100,
            y: 100,
            width: 1280,
            height: 800,
        }
    }
}

/// Window configuration state saved across application launches.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowConfig {
    /// Window title.
    pub title: String,
    /// Dimensions and screen placement.
    pub dimensions: WindowDimensions,
    /// Whether the window was maximized.
    pub is_maximized: bool,
    /// Whether the window is in borderless fullscreen mode.
    pub is_fullscreen: bool,
    /// Minimum allowed window width.
    pub min_width: u32,
    /// Minimum allowed window height.
    pub min_height: u32,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "Actual Budget".into(),
            dimensions: WindowDimensions::default(),
            is_maximized: false,
            is_fullscreen: false,
            min_width: 800,
            min_height: 600,
        }
    }
}

impl WindowConfig {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            ..Default::default()
        }
    }

    /// Serializes configuration to a JSON string for persistence in user config directory.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserializes configuration from JSON string.
    pub fn from_json(json_str: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json_str)
    }
}
