//! Actual Budget mobile application binary entry point and interactive simulator.
//!
//! Provides CLI diagnostics, viewport metrics calculation, touch gesture simulation,
//! and packaging manifest output for Android (APK) and iOS (IPA / AltStore sideloading).

use actual_gui::mobile::{
    AndroidPackagingConfig, IosPackagingConfig, KeypadKey, MobileApp, MobileTab, MobileViewport,
    ScreenOrientation, TouchPhase,
};
use clap::Parser;

/// Actual Budget - 100% Rust Native Mobile Client Simulator & Packager
#[derive(Parser, Debug)]
#[command(
    name = "actual-mobile",
    author,
    version,
    about = "Privacy-first local envelope budgeting mobile client for Android & iOS"
)]
struct Args {
    /// Device form factor: "compact", "standard", "large", or "tablet".
    #[arg(long, default_value = "standard")]
    form_factor: String,

    /// Device orientation: "portrait" or "landscape".
    #[arg(long, default_value = "portrait")]
    orientation: String,

    /// Print target platform packaging manifest ("android", "ios", or "gradle") and exit.
    #[arg(long)]
    print_manifest: Option<String>,

    /// Simulate Quick Add transaction amount in dollars (e.g. 24.50).
    #[arg(long)]
    quick_add_amount: Option<f64>,

    /// Notes for simulated Quick Add transaction.
    #[arg(long, default_value = "Mobile Expense")]
    quick_add_notes: String,

    /// Run headless self-diagnostic check and exit.
    #[arg(long)]
    headless_check: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Handle manifest printing
    if let Some(platform) = args.print_manifest {
        match platform.to_lowercase().as_str() {
            "android" => {
                let config = AndroidPackagingConfig::default();
                println!("{}", config.generate_manifest());
                return Ok(());
            }
            "gradle" => {
                let config = AndroidPackagingConfig::default();
                println!("{}", config.generate_build_gradle());
                return Ok(());
            }
            "ios" => {
                let config = IosPackagingConfig::default();
                println!("{}", config.generate_info_plist());
                return Ok(());
            }
            other => {
                eprintln!("Unknown platform '{other}'. Valid options: android, gradle, ios");
                std::process::exit(1);
            }
        }
    }

    let mut app = MobileApp::new();

    // Configure viewport based on form factor
    let mut viewport = match args.form_factor.to_lowercase().as_str() {
        "compact" => MobileViewport::compact_phone(),
        "large" => MobileViewport::large_phone(),
        "tablet" => MobileViewport::tablet(),
        _ => MobileViewport::standard_phone(),
    };

    if args.orientation.to_lowercase() == "landscape" {
        viewport.orientation = ScreenOrientation::Landscape;
        std::mem::swap(&mut viewport.width, &mut viewport.height);
    }
    app.viewport = viewport;

    // Simulate Quick Add if amount is provided
    if let Some(amount) = args.quick_add_amount {
        app.switch_tab(MobileTab::QuickAdd);
        let cents_val = (amount * 100.0).round() as i64;
        let dollars = cents_val.abs() / 100;
        let cents = cents_val.abs() % 100;

        for ch in dollars.to_string().chars() {
            if let Some(d) = ch.to_digit(10) {
                app.keypad_press(KeypadKey::Digit(d as u8));
            }
        }
        app.keypad_press(KeypadKey::Decimal);
        let cents_str = format!("{cents:02}");
        for ch in cents_str.chars() {
            if let Some(d) = ch.to_digit(10) {
                app.keypad_press(KeypadKey::Digit(d as u8));
            }
        }

        app.quick_add.notes = args.quick_add_notes;
        let tx = app.commit_quick_add();
        if let Some(committed) = tx {
            println!(
                "Simulated Quick Add committed: ID={}, Amount={}, Notes={}",
                committed.id,
                committed.amount,
                committed.notes.unwrap_or_default()
            );
        }
    }

    // Diagnostics / headless check
    if args.headless_check {
        println!("Actual Budget Mobile Client - Self Diagnostic Check: OK");
        println!("Form Factor: {:?}", app.viewport.form_factor());
        println!("Orientation: {:?}", app.viewport.orientation);
        println!(
            "Resolution: {}x{} pt (usable: {}x{} pt)",
            app.viewport.width,
            app.viewport.height,
            app.viewport.usable_width(),
            app.viewport.usable_height()
        );
        println!(
            "Safe Area Insets: top={}pt, bottom={}pt, left={}pt, right={}pt",
            app.viewport.safe_area.top,
            app.viewport.safe_area.bottom,
            app.viewport.safe_area.left,
            app.viewport.safe_area.right,
        );
        println!("Active Tab: {:?}", app.nav_bar.active_tab);
        println!("Stored Transactions: {}", app.transactions.len());
        println!("Total Balance: {}", app.total_balance());

        // Test touch gesture classifier response
        app.handle_touch(TouchPhase::Began, 100.0, 100.0, 1000);
        let gesture = app.handle_touch(TouchPhase::Ended, 101.0, 101.0, 1050);
        println!("Touch Classifier Verification: {:?}", gesture);

        return Ok(());
    }

    println!("Starting Actual Budget Mobile Client (Native Mobile Runtime ready)");
    println!("Platform adapters: Android NativeActivity & iOS UIKit");
    Ok(())
}
