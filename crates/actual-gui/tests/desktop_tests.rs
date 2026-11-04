//! Integration tests for actual-gui desktop client: window configuration, menus,
//! shortcuts, native theme detection, local SQLite persistence, background sync, and OS packaging.

use actual_api::commands::{CreateAccount, CreateCategory, CreateCategoryGroup, SetBudget};
use actual_core::cents::Cents;
use actual_gui::desktop::{
    detect_system_dark_mode, ActiveTab, ColorPalette, DesktopApp, LinuxPackaging, MacOsPackaging,
    MenuAction, MenuBar, Shortcut, ShortcutModifier, SyncConfig, ThemeMode, WindowConfig,
    WindowsPackaging,
};
use actual_sync::queue::SyncStatus;
use tempfile::tempdir;

#[test]
fn test_window_config_serialization_and_defaults() {
    let mut config = WindowConfig::default();
    assert_eq!(config.title, "Actual Budget");
    assert_eq!(config.dimensions.width, 1280);
    assert_eq!(config.dimensions.height, 800);
    assert!(!config.is_fullscreen);
    assert!(!config.is_maximized);

    config.dimensions.width = 1600;
    config.dimensions.height = 1000;
    config.is_maximized = true;

    let json = config.to_json().expect("serialize window config");
    let restored = WindowConfig::from_json(&json).expect("deserialize window config");
    assert_eq!(restored, config);
}

#[test]
fn test_menu_bar_structure_and_shortcuts() {
    let menu = MenuBar::standard_menu();
    assert_eq!(menu.categories.len(), 5);
    assert_eq!(menu.categories[0].title, "File");
    assert_eq!(menu.categories[1].title, "Edit");
    assert_eq!(menu.categories[2].title, "View");
    assert_eq!(menu.categories[3].title, "Sync");
    assert_eq!(menu.categories[4].title, "Help");

    // Test shortcut display string
    let sc = Shortcut::new(ShortcutModifier::CTRL_SHIFT, "Z");
    let label = sc.display_label();
    assert!(label.contains("Shift"));
    assert!(label.contains("Z"));
}

#[test]
fn test_theme_mode_and_palette_resolution() {
    let dark_palette = ColorPalette::resolve(ThemeMode::Dark, false);
    assert_eq!(dark_palette.background, "#181b21");
    assert_eq!(dark_palette.surface, "#242933");

    let light_palette = ColorPalette::resolve(ThemeMode::Light, true);
    assert_eq!(light_palette.background, "#f6f8fa");
    assert_eq!(light_palette.surface, "#ffffff");

    // System mode reflects system flag
    let sys_dark = ColorPalette::resolve(ThemeMode::System, true);
    assert_eq!(sys_dark.background, "#181b21");

    let sys_light = ColorPalette::resolve(ThemeMode::System, false);
    assert_eq!(sys_light.background, "#f6f8fa");

    // Theme detection runs safely
    let _ = detect_system_dark_mode();
}

#[test]
fn test_desktop_app_file_persistence_lifecycle() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("my-test-budget.sqlite");

    // 1. First launch: create budget file and populate initial data
    {
        let mut app = DesktopApp::open_file(&db_path).expect("open app file");
        assert_eq!(app.budget_path, Some(db_path.clone()));

        // Add Category Group & Category
        let grp_id = app
            .bridge
            .service
            .create_category_group(CreateCategoryGroup {
                id: String::new(),
                name: "Monthly Bills".into(),
                is_income: false,
            })
            .unwrap();

        let cat_id = app
            .bridge
            .service
            .create_category(CreateCategory {
                id: String::new(),
                group_id: grp_id,
                name: "Internet & Wi-Fi".into(),
                is_income: false,
            })
            .unwrap();

        app.bridge
            .service
            .set_budget(SetBudget {
                category_id: cat_id,
                month: "2024-05".into(),
                amount: Cents::from_cents(8000),
            })
            .unwrap();

        app.bridge
            .refresh_budget_view(&mut app.budget_view)
            .unwrap();
        assert_eq!(app.budget_view.table.row_count(), 1);
    }

    // 2. Second launch: reopen same file from disk and verify persistence
    {
        let mut app = DesktopApp::open_file(&db_path).expect("reopen app file");
        app.budget_view.current_month = "2024-05".into();
        app.bridge
            .refresh_budget_view(&mut app.budget_view)
            .unwrap();

        assert_eq!(app.budget_view.table.row_count(), 1);
        let cell_name = app.budget_view.table.get_cell(0, 0).unwrap();
        assert_eq!(cell_name.value.display_string(), "Internet & Wi-Fi");

        let cell_budget = app.budget_view.table.get_cell(0, 1).unwrap();
        assert_eq!(cell_budget.value.display_string(), "80.00");
    }
}

#[test]
fn test_desktop_app_menu_action_dispatch() {
    let mut app = DesktopApp::open_in_memory().expect("open in-memory app");

    // Add account
    app.bridge
        .service
        .create_account(CreateAccount {
            id: String::new(),
            name: "Checking".into(),
            offbudget: false,
        })
        .unwrap();

    // Toggle Theme
    let init_theme = app.theme_mode;
    let handled = app.dispatch_action(MenuAction::ToggleTheme).unwrap();
    assert!(handled);
    assert_ne!(app.theme_mode, init_theme);

    // Toggle Fullscreen
    assert!(!app.window_config.is_fullscreen);
    app.dispatch_action(MenuAction::ToggleFullscreen).unwrap();
    assert!(app.window_config.is_fullscreen);

    // Find Transaction switches active tab
    assert_eq!(app.active_tab, ActiveTab::Budget);
    app.dispatch_action(MenuAction::FindTransaction).unwrap();
    assert_eq!(app.active_tab, ActiveTab::Transactions);
}

#[test]
fn test_desktop_background_sync_configuration() {
    let mut app = DesktopApp::open_in_memory().expect("open in-memory app");

    app.configure_sync(SyncConfig {
        server_url: "http://127.0.0.1:5006".into(),
        file_id: "test-file-uuid".into(),
        group_id: "test-group-uuid".into(),
        auto_sync_interval_secs: 15,
    });

    assert!(app.sync_worker.is_some());
    let worker = app.sync_worker.as_ref().unwrap();
    assert_eq!(worker.config.auto_sync_interval_secs, 15);

    let state = worker.current_state();
    assert_eq!(state.status, SyncStatus::Synced);
    assert_eq!(state.failure_count, 0);
}

#[test]
fn test_desktop_packaging_metadata_validation() {
    // 1. Linux packaging
    let desktop_entry = LinuxPackaging::generate_desktop_entry();
    assert!(desktop_entry.contains("[Desktop Entry]"));
    assert!(desktop_entry.contains("Name=Actual Budget"));
    assert!(desktop_entry.contains("Exec=actual-desktop %f"));
    assert!(desktop_entry.contains("Categories=Office;Finance;"));
    assert!(desktop_entry.contains("MimeType=application/x-actual-budget;"));

    let appstream = LinuxPackaging::generate_appstream_metainfo();
    assert!(appstream.contains("<id>com.actualbudget.actual</id>"));
    assert!(appstream.contains("<name>Actual Budget</name>"));

    // 2. macOS packaging
    let info_plist = MacOsPackaging::generate_info_plist();
    assert!(info_plist.contains("<key>CFBundleIdentifier</key>"));
    assert!(info_plist.contains("<string>com.actualbudget.actual</string>"));
    assert!(info_plist.contains("<key>CFBundleExecutable</key>"));
    assert!(info_plist.contains("<string>actual-desktop</string>"));
    assert!(info_plist.contains("<key>NSRequiresAquaSystemAppearance</key>"));

    // 3. Windows packaging
    let nsis_script = WindowsPackaging::generate_nsis_script();
    assert!(nsis_script.contains("!define APP_NAME \"Actual Budget\""));
    assert!(nsis_script.contains("actual-desktop.exe"));
    assert!(nsis_script.contains("CreateShortCut"));

    let manifest = WindowsPackaging::generate_app_manifest();
    assert!(manifest.contains("PerMonitorV2"));
}
