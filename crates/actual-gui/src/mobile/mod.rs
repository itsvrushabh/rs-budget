//! 100% Rust mobile application architecture for Android and iOS devices.
//!
//! Provides mobile-responsive viewport layouts, safe area insets, touch gestures,
//! financial keypad entry, pull-to-refresh sync, and Android/iOS packaging definitions.

pub mod app;
pub mod gestures;
pub mod keypad;
pub mod layout;
pub mod packaging;

pub use app::MobileApp;
pub use gestures::{Gesture, GestureClassifier, TouchPhase, TouchPoint};
pub use keypad::{KeypadKey, KeypadState, QuickAddSheet};
pub use layout::{
    BottomNavBar, DeviceFormFactor, MobileTab, MobileViewport, SafeAreaInsets, ScreenOrientation,
};
pub use packaging::{AndroidPackagingConfig, IosPackagingConfig};
