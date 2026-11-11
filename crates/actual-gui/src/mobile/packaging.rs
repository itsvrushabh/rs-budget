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

    /// Generates Android ProGuard/R8 obfuscation and keep rules.
    pub fn generate_proguard_rules(&self) -> String {
        r#"# Proguard rules for Actual Budget Mobile (NativeActivity)
-keep class android.app.NativeActivity { *; }
-keepclassmembers class * extends android.app.NativeActivity {
    *;
}
-dontwarn android.app.NativeActivity
"#
        .to_string()
    }

    /// Returns direct sideloading instructions for Android APK installation.
    pub fn generate_sideload_guide(&self) -> String {
        format!(
            "# Android APK Direct Installation Guide\n\n\
            1. Download `{package_name}.apk` directly from the GitHub Releases page.\n\
            2. On your Android device, go to **Settings > Security** (or Apps & Notifications) and enable **Install Unknown Apps** for your browser or file manager.\n\
            3. Tap the downloaded `.apk` file and tap **Install**.\n\
            4. Open **{app_name}** and start budgeting!\n",
            package_name = self.package_name,
            app_name = self.app_name,
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

    /// Generates legacy macOS/iOS PkgInfo bundle marker.
    pub fn generate_pkg_info(&self) -> String {
        "APPL????".to_string()
    }

    /// Generates ExportOptions.plist for Xcode ad-hoc/development distribution.
    pub fn generate_export_options_plist(&self) -> String {
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>method</key>
    <string>development</string>
    <key>compileBitcode</key>
    <false/>
    <key>signingStyle</key>
    <string>automatic</string>
    <key>stripSwiftSymbols</key>
    <true/>
</dict>
</plist>
"#
        .to_string()
    }

    /// Returns direct sideloading instructions for iOS without Apple Developer Program membership.
    pub fn generate_sideload_guide(&self) -> String {
        format!(
            "# iOS Direct Installation Guide (No Apple Developer Program Membership Required)\n\n\
            You do NOT need a paid $99/year Apple Developer account to install `{app_name}` on your iPhone or iPad.\n\n\
            ### Method 1: AltStore / SideStore (Recommended)\n\
            1. Install [AltServer](https://altstore.io) on your Mac or Windows PC.\n\
            2. Connect your iPhone/iPad via USB, install AltStore onto your device using your free personal Apple ID.\n\
            3. Download `{bundle_name}-unsigned.ipa` from GitHub Releases onto your iPhone/iPad.\n\
            4. In the AltStore app on your device, tap **+** (My Apps) and select the downloaded `.ipa`.\n\
            5. AltStore will sign and install the app directly to your home screen.\n\n\
            ### Method 2: Sideloadly (PC or Mac)\n\
            1. Download [Sideloadly](https://sideloadly.io) on Windows or macOS.\n\
            2. Connect your iPhone/iPad via USB.\n\
            3. Drag and drop `{bundle_name}-unsigned.ipa` into Sideloadly.\n\
            4. Enter your free Apple ID and click **Start**.\n\
            5. On your iPhone, go to **Settings > General > VPN & Device Management** and tap **Trust [Your Apple ID]**.\n\n\
            ### Method 3: TrollStore (iOS 14.0 - 16.6.1 / 17.0)\n\
            1. If your device supports TrollStore, simply share `{bundle_name}-unsigned.ipa` to TrollStore.\n\
            2. TrollStore signs the app permanently with no 7-day expiration.\n\n\
            ### Method 4: Xcode Free Personal Team (Mac only)\n\
            1. Open Xcode on macOS, log in with your free Apple ID under **Settings > Accounts**.\n\
            2. Create or open the project, select your Personal Team under **Signing & Capabilities**.\n\
            3. Connect your iPhone or iPad, select it as run destination, and click **Run** (Cmd+R).\n",
            app_name = self.bundle_display_name,
            bundle_name = self.bundle_name,
        )
    }
}
