//! Standalone installer, bundle, and metadata specifications for Linux, macOS, and Windows.

use serde::{Deserialize, Serialize};

/// Target operating system for packaging.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OsTarget {
    Linux,
    MacOs,
    Windows,
}

/// Linux FreeDesktop specification generator (`.desktop` entry and AppStream XML).
pub struct LinuxPackaging;

impl LinuxPackaging {
    /// Generates FreeDesktop `.desktop` application launcher file.
    #[must_use]
    pub fn generate_desktop_entry() -> String {
        r#"[Desktop Entry]
Name=Actual Budget
GenericName=Personal Finance & Budgeting
Comment=Privacy-first, local-first envelope budgeting application built in 100% Rust
Exec=actual-desktop %f
Icon=actual-budget
Terminal=false
Type=Application
Categories=Office;Finance;
MimeType=application/x-actual-budget;
StartupWMClass=actual-desktop
Keywords=finance;budget;envelope;money;accounting;
"#
        .to_string()
    }

    /// Generates AppStream Metadata XML for software centers (GNOME Software, KDE Discover).
    #[must_use]
    pub fn generate_appstream_metainfo() -> String {
        r#"<?xml version="1.0" encoding="UTF-8"?>
<component type="desktop-application">
  <id>com.actualbudget.actual</id>
  <metadata_license>CC0-1.0</metadata_license>
  <project_license>MIT</project_license>
  <name>Actual Budget</name>
  <summary>Privacy-focused, 100% Rust local-first envelope budgeting app</summary>
  <description>
    <p>
      Actual Budget is a 100% pure Rust rewrite of the popular personal finance system,
      delivering local-first SQLite persistence, end-to-end AES-256-GCM encryption,
      and conflict-free CRDT synchronization with self-hosted Axum servers.
    </p>
  </description>
  <launchable type="desktop-id">actual.desktop</launchable>
  <provides>
    <mediatype>application/x-actual-budget</mediatype>
  </provides>
</component>
"#
        .to_string()
    }
}

/// macOS Application Bundle specification generator (`Info.plist`).
pub struct MacOsPackaging;

impl MacOsPackaging {
    /// Generates standard macOS `Info.plist` bundle property list.
    #[must_use]
    pub fn generate_info_plist() -> String {
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleIdentifier</key>
    <string>com.actualbudget.actual</string>
    <key>CFBundleName</key>
    <string>Actual Budget</string>
    <key>CFBundleExecutable</key>
    <string>actual-desktop</string>
    <key>CFBundleIconFile</key>
    <string>AppIcon.icns</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>0.1.0</string>
    <key>CFBundleVersion</key>
    <string>1</string>
    <key>LSMinimumSystemVersion</key>
    <string>11.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSRequiresAquaSystemAppearance</key>
    <false/>
</dict>
</plist>
"#
        .to_string()
    }
}

/// Windows installer script generator (NSIS / WiX).
pub struct WindowsPackaging;

impl WindowsPackaging {
    /// Generates Nullsoft Scriptable Install System (NSIS) installer script.
    #[must_use]
    pub fn generate_nsis_script() -> String {
        r#"; NSIS Installer script for Actual Budget (Windows)
!define APP_NAME "Actual Budget"
!define APP_VERSION "0.1.0"
!define APP_PUBLISHER "Actual Budget Contributors"
!define APP_EXE "actual-desktop.exe"

Name "${APP_NAME}"
OutFile "actual-budget-setup.exe"
InstallDir "$PROGRAMFILES64\Actual Budget"
RequestExecutionLevel admin

Section "MainSection" SEC01
    SetOutPath "$INSTDIR"
    File "target\release\actual-desktop.exe"
    CreateDirectory "$SMPROGRAMS\Actual Budget"
    CreateShortCut "$SMPROGRAMS\Actual Budget\Actual Budget.lnk" "$INSTDIR\actual-desktop.exe"
    CreateShortCut "$DESKTOP\Actual Budget.lnk" "$INSTDIR\actual-desktop.exe"
SectionEnd
"#
        .to_string()
    }

    /// Generates Windows Application Manifest declaring Per-Monitor V2 DPI Awareness.
    #[must_use]
    pub fn generate_app_manifest() -> String {
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <application xmlns="urn:schemas-microsoft-com:asm.v3">
    <windowsSettings>
      <dpiAware xmlns="http://schemas.microsoft.com/SMI/2005/WindowsSettings">true/pm</dpiAware>
      <dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">PerMonitorV2, PerMonitor</dpiAwareness>
    </windowsSettings>
  </application>
</assembly>
"#
        .to_string()
    }
}
