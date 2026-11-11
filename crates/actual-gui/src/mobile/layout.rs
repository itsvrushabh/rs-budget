//! Mobile-responsive viewport layouts, safe area insets, and bottom navigation bar.
//!
//! Provides adaptive mobile UI layout primitives tailored for iOS and Android form factors,
//! including notch/Dynamic Island, status bars, and home indicator safe zones.

use serde::{Deserialize, Serialize};

/// Mobile device device class / form factor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceFormFactor {
    CompactPhone,  // e.g. iPhone SE, Pixel 4a (< 390pt width)
    StandardPhone, // e.g. iPhone 14/15/16, Galaxy S23 (390..=430pt width)
    LargePhone,    // e.g. iPhone Pro Max, Galaxy Ultra (> 430pt width)
    Tablet,        // e.g. iPad, Galaxy Tab (>= 600pt width)
}

/// Screen orientation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScreenOrientation {
    Portrait,
    Landscape,
}

/// Safe Area Insets in logical pixels (pt/dp).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SafeAreaInsets {
    pub top: u32,
    pub bottom: u32,
    pub left: u32,
    pub right: u32,
}

impl Default for SafeAreaInsets {
    fn default() -> Self {
        Self {
            top: 47,    // Typical iPhone notch / Dynamic Island height
            bottom: 34, // Home indicator bar safe zone
            left: 0,
            right: 0,
        }
    }
}

/// Mobile viewport dimensions and layout parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MobileViewport {
    pub width: u32,
    pub height: u32,
    pub scale_factor: u32, // e.g. 2x, 3x retina
    pub orientation: ScreenOrientation,
    pub safe_area: SafeAreaInsets,
}

impl MobileViewport {
    /// Compact iPhone / Android (375 x 667 logical pt, 2x scale, e.g. iPhone SE).
    pub fn compact_phone() -> Self {
        Self {
            width: 375,
            height: 667,
            scale_factor: 2,
            orientation: ScreenOrientation::Portrait,
            safe_area: SafeAreaInsets {
                top: 20,
                bottom: 0,
                left: 0,
                right: 0,
            },
        }
    }

    /// Standard iPhone (393 x 852 logical pt, 3x scale).
    pub fn standard_phone() -> Self {
        Self {
            width: 393,
            height: 852,
            scale_factor: 3,
            orientation: ScreenOrientation::Portrait,
            safe_area: SafeAreaInsets::default(),
        }
    }

    /// Large Phone (430 x 932 logical pt, 3x scale, e.g. iPhone Pro Max, Galaxy Ultra).
    pub fn large_phone() -> Self {
        Self {
            width: 430,
            height: 932,
            scale_factor: 3,
            orientation: ScreenOrientation::Portrait,
            safe_area: SafeAreaInsets {
                top: 59,
                bottom: 34,
                left: 0,
                right: 0,
            },
        }
    }

    /// iPad / Tablet (820 x 1180 logical pt, 2x scale).
    pub fn tablet() -> Self {
        Self {
            width: 820,
            height: 1180,
            scale_factor: 2,
            orientation: ScreenOrientation::Portrait,
            safe_area: SafeAreaInsets {
                top: 24,
                bottom: 20,
                left: 0,
                right: 0,
            },
        }
    }

    /// Determines device form factor from viewport width.
    pub fn form_factor(&self) -> DeviceFormFactor {
        if self.width >= 600 {
            DeviceFormFactor::Tablet
        } else if self.width >= 430 {
            DeviceFormFactor::LargePhone
        } else if self.width >= 390 {
            DeviceFormFactor::StandardPhone
        } else {
            DeviceFormFactor::CompactPhone
        }
    }

    /// Usable content area height excluding safe areas.
    pub fn usable_height(&self) -> u32 {
        self.height
            .saturating_sub(self.safe_area.top + self.safe_area.bottom)
    }

    /// Usable content area width excluding safe areas.
    pub fn usable_width(&self) -> u32 {
        self.width
            .saturating_sub(self.safe_area.left + self.safe_area.right)
    }
}

/// Mobile bottom navigation bar items.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MobileTab {
    Budget,
    Accounts,
    QuickAdd,
    Reports,
    Settings,
}

/// Mobile navigation bar state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BottomNavBar {
    pub active_tab: MobileTab,
    pub unread_sync_count: u32,
}

impl Default for BottomNavBar {
    fn default() -> Self {
        Self {
            active_tab: MobileTab::Budget,
            unread_sync_count: 0,
        }
    }
}

impl BottomNavBar {
    /// Selects an active tab.
    pub fn select_tab(&mut self, tab: MobileTab) {
        self.active_tab = tab;
    }
}
