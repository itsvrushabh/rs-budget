//! Integration tests for 100% Rust mobile application architecture on Android and iOS.

use actual_core::cents::Cents;
use actual_gui::mobile::{
    AndroidPackagingConfig, DeviceFormFactor, Gesture, GestureClassifier, IosPackagingConfig,
    KeypadKey, KeypadState, MobileApp, MobileTab, MobileViewport, TouchPhase,
};

#[test]
fn test_mobile_viewport_and_safe_area() {
    let phone = MobileViewport::standard_phone();
    assert_eq!(phone.form_factor(), DeviceFormFactor::StandardPhone);
    assert_eq!(phone.width, 393);
    assert_eq!(phone.height, 852);
    assert_eq!(phone.safe_area.top, 47);
    assert_eq!(phone.safe_area.bottom, 34);
    assert_eq!(phone.usable_width(), 393);
    assert_eq!(phone.usable_height(), 852 - 47 - 34);

    let tablet = MobileViewport::tablet();
    assert_eq!(tablet.form_factor(), DeviceFormFactor::Tablet);
    assert_eq!(tablet.width, 820);
    assert_eq!(tablet.height, 1180);
    assert_eq!(tablet.usable_width(), 820);
    assert_eq!(tablet.usable_height(), 1180 - 24 - 20);
}

#[test]
fn test_mobile_tap_double_tap_long_press_gestures() {
    let mut classifier = GestureClassifier::default();

    // 1. Single Tap
    classifier.process_touch(TouchPhase::Began, 100.0, 150.0, 1000);
    let tap = classifier.process_touch(TouchPhase::Ended, 102.0, 151.0, 1100);
    assert_eq!(tap, Some(Gesture::Tap { x: 102.0, y: 151.0 }));

    // 2. Double Tap within 350ms
    classifier.process_touch(TouchPhase::Began, 100.0, 150.0, 1200);
    let double_tap = classifier.process_touch(TouchPhase::Ended, 101.0, 150.0, 1250);
    assert_eq!(double_tap, Some(Gesture::DoubleTap { x: 101.0, y: 150.0 }));

    // 3. Long Press >= 500ms
    classifier.process_touch(TouchPhase::Began, 200.0, 200.0, 2000);
    let long_press = classifier.process_touch(TouchPhase::Ended, 201.0, 201.0, 2600);
    match long_press {
        Some(Gesture::LongPress { duration_ms, .. }) => {
            assert!(duration_ms >= 500);
        }
        other => panic!("Expected LongPress, got {other:?}"),
    }
}

#[test]
fn test_mobile_swipe_gestures() {
    let mut classifier = GestureClassifier::default();

    // Swipe Left (> 60pt horizontal distance)
    classifier.process_touch(TouchPhase::Began, 300.0, 200.0, 1000);
    let swipe_left = classifier.process_touch(TouchPhase::Ended, 200.0, 205.0, 1150);
    match swipe_left {
        Some(Gesture::SwipeLeft { distance, velocity }) => {
            assert!(distance >= 60.0);
            assert!(velocity > 0.0);
        }
        other => panic!("Expected SwipeLeft, got {other:?}"),
    }

    // Swipe Right
    classifier.process_touch(TouchPhase::Began, 100.0, 200.0, 2000);
    let swipe_right = classifier.process_touch(TouchPhase::Ended, 220.0, 202.0, 2120);
    match swipe_right {
        Some(Gesture::SwipeRight { distance, velocity }) => {
            assert!(distance >= 60.0);
            assert!(velocity > 0.0);
        }
        other => panic!("Expected SwipeRight, got {other:?}"),
    }
}

#[test]
fn test_mobile_pull_to_refresh_gesture() {
    let mut classifier = GestureClassifier::default();

    classifier.process_touch(TouchPhase::Began, 200.0, 100.0, 1000);
    // Vertical drag downwards by 95pt
    let pull = classifier.process_touch(TouchPhase::Moved, 202.0, 195.0, 1100);
    match pull {
        Some(Gesture::PullToRefresh { distance_y }) => {
            assert!(distance_y >= 80.0);
        }
        other => panic!("Expected PullToRefresh, got {other:?}"),
    }
}

#[test]
fn test_mobile_keypad_amount_entry() {
    let mut keypad = KeypadState::new();
    assert_eq!(keypad.raw_buffer, "0");
    assert!(keypad.is_negative); // Default expense

    // Type 45.50
    keypad.press_key(KeypadKey::Digit(4));
    keypad.press_key(KeypadKey::Digit(5));
    keypad.press_key(KeypadKey::Decimal);
    keypad.press_key(KeypadKey::Digit(5));
    keypad.press_key(KeypadKey::Digit(0));

    assert_eq!(keypad.raw_buffer, "45.50");
    assert_eq!(keypad.to_cents(), Cents(-4550));
    assert_eq!(keypad.display_string(), "-$45.50");

    // Toggle income with Plus
    keypad.press_key(KeypadKey::Plus);
    assert!(!keypad.is_negative);
    assert_eq!(keypad.to_cents(), Cents(4550));
    assert_eq!(keypad.display_string(), "+$45.50");

    // Backspace
    keypad.press_key(KeypadKey::Backspace);
    assert_eq!(keypad.raw_buffer, "45.5");

    // Clear
    keypad.press_key(KeypadKey::Clear);
    assert_eq!(keypad.raw_buffer, "0");
    assert_eq!(keypad.to_cents(), Cents(0));
}

#[test]
fn test_mobile_quick_add_transaction_flow() {
    let mut app = MobileApp::new();
    assert_eq!(app.transactions.len(), 0);

    // Tap QuickAdd tab
    app.switch_tab(MobileTab::QuickAdd);
    assert!(app.quick_add.is_open);

    // Enter $25.00 via keypad
    app.keypad_press(KeypadKey::Digit(2));
    app.keypad_press(KeypadKey::Digit(5));
    app.quick_add.notes = "Coffee & Bakery".to_string();

    // Commit quick add
    let tx = app.commit_quick_add().unwrap();
    assert_eq!(tx.amount, Cents(-2500));
    assert_eq!(tx.notes, Some("Coffee & Bakery".to_string()));
    assert_eq!(app.transactions.len(), 1);
    assert_eq!(app.total_balance(), Cents(-2500));
    assert!(!app.quick_add.is_open);

    // Swipe left gesture removes/deletes transaction
    app.handle_touch(TouchPhase::Began, 300.0, 100.0, 1000);
    app.handle_touch(TouchPhase::Ended, 150.0, 102.0, 1150);
    assert_eq!(app.transactions.len(), 0);
}

#[test]
fn test_mobile_packaging_manifests() {
    // Android Manifest & Gradle
    let android = AndroidPackagingConfig::default();
    let manifest_xml = android.generate_manifest();
    assert!(manifest_xml.contains("package=\"org.actualbudget.mobile\""));
    assert!(manifest_xml.contains("android.permission.INTERNET"));
    assert!(manifest_xml.contains("NativeActivity"));

    let gradle = android.generate_build_gradle();
    assert!(gradle.contains("applicationId 'org.actualbudget.mobile'"));
    assert!(gradle.contains("compileSdk 34"));

    // iOS Info.plist
    let ios = IosPackagingConfig::default();
    let plist_xml = ios.generate_info_plist();
    assert!(plist_xml.contains("<key>CFBundleIdentifier</key>"));
    assert!(plist_xml.contains("<string>org.actualbudget.mobile</string>"));
    assert!(plist_xml.contains("UIInterfaceOrientationPortrait"));
}
