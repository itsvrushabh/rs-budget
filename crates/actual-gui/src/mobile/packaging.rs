//! Android and iOS packaging specifications, manifest generators, and bundle definitions.
//!
//! Generates compliant `AndroidManifest.xml` for Google Play / APK and `Info.plist`
//! for iOS / iPadOS IPA TestFlight distribution.

use serde::{Deserialize, Serialize};

/// Android packaging metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AndroidPackagingConfig {
    pub package_name: String,
    pub app_name: String,
    pub version_code: u32,
    pub version_name: String,
    pub min_sdk: u32,
    pub target_sdk: u32,
}

impl Default for AndroidPackagingConfig {
    fn default() -> Self {
        Self {
            package_name: "org.actualbudget.mobile".to_string(),
            app_name: "Actual Budget".to_string(),
            version_code: 1,
            version_name: "0.1.0".to_string(),
            min_sdk: 26,    // Android 8.0 Oreo minimum
            target_sdk: 34, // Android 14
        }
    }
}

impl AndroidPackagingConfig {
    /// Generates AndroidManifest.xml string.
    pub fn generate_manifest(&self) -> String {
        format!(
            r#"<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
    package="{package_name}"
    android:versionCode="{version_code}"
    android:versionName="{version_name}">

    <uses-permission android:name="android.permission.INTERNET" />
    <uses-permission android:name="android.permission.ACCESS_NETWORK_STATE" />

    <application
        android:label="{app_name}"
        android:hasCode="false"
        android:hardwareAccelerated="true"
        android:icon="@mipmap/ic_launcher"
        android:roundIcon="@mipmap/ic_launcher_round"
        android:supportsRtl="true"
        android:theme="@android:style/Theme.NoTitleBar.Fullscreen">

        <meta-data android:name="android.app.lib_name" android:value="actual_mobile" />

        <activity
            android:name="android.app.NativeActivity"
            android:label="{app_name}"
            android:configChanges="orientation|keyboardHidden|screenSize"
            android:exported="true">
            <intent-filter>
                <action android:name="android.intent.action.MAIN" />
                <category android:name="android.intent.category.LAUNCHER" />
            </intent-filter>
        </activity>
    </application>
</manifest>
"#,
            package_name = self.package_name,
            version_code = self.version_code,
            version_name = self.version_name,
            app_name = self.app_name,
        )
    }

    /// Generates minimal Android build.gradle script.
    pub fn generate_build_gradle(&self) -> String {
        format!(
            r#"plugins {{
    id 'com.android.application'
}}

android {{
    namespace '{package_name}'
    compileSdk {target_sdk}

    defaultConfig {{
        applicationId '{package_name}'
        minSdk {min_sdk}
        targetSdk {target_sdk}
        versionCode {version_code}
        versionName '{version_name}'
    }}

    buildTypes {{
        release {{
            minifyEnabled true
            proguardFiles getDefaultProguardFile('proguard-android-optimize.txt'), 'proguard-rules.pro'
        }}
    }}
}}
"#,
            package_name = self.package_name,
            min_sdk = self.min_sdk,
            target_sdk = self.target_sdk,
            version_code = self.version_code,
            version_name = self.version_name,
        )
    }
}

/// iOS packaging metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IosPackagingConfig {
    pub bundle_identifier: String,
    pub bundle_name: String,
    pub bundle_display_name: String,
    pub bundle_version: String,
    pub bundle_short_version: String,
}

impl Default for IosPackagingConfig {
    fn default() -> Self {
        Self {
            bundle_identifier: "org.actualbudget.mobile".to_string(),
            bundle_name: "ActualMobile".to_string(),
            bundle_display_name: "Actual Budget".to_string(),
            bundle_version: "1".to_string(),
            bundle_short_version: "0.1.0".to_string(),
        }
    }
}

impl IosPackagingConfig {
    /// Generates Info.plist XML for iOS app bundle.
    pub fn generate_info_plist(&self) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleExecutable</key>
    <string>{bundle_name}</string>
    <key>CFBundleIdentifier</key>
    <string>{bundle_identifier}</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>{bundle_name}</string>
    <key>CFBundleDisplayName</key>
    <string>{bundle_display_name}</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>{bundle_short_version}</string>
    <key>CFBundleVersion</key>
    <string>{bundle_version}</string>
    <key>LSRequiresIPhoneOS</key>
    <true/>
    <key>UILaunchStoryboardName</key>
    <string>LaunchScreen</string>
    <key>UIRequiredDeviceCapabilities</key>
    <array>
        <string>arm64</string>
    </array>
    <key>UISupportedInterfaceOrientations</key>
    <array>
        <string>UIInterfaceOrientationPortrait</string>
        <string>UIInterfaceOrientationLandscapeLeft</string>
        <string>UIInterfaceOrientationLandscapeRight</string>
    </array>
    <key>UIViewControllerBasedStatusBarAppearance</key>
    <false/>
</dict>
</plist>
"#,
            bundle_name = self.bundle_name,
            bundle_identifier = self.bundle_identifier,
            bundle_display_name = self.bundle_display_name,
            bundle_short_version = self.bundle_short_version,
            bundle_version = self.bundle_version,
        )
    }
}
