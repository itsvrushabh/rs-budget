# Mobile Installation Guide: Android & iOS

`rs-budget` features a native, local-first mobile client (`actual-mobile`) written in 100% Rust. This guide covers how to install and run the app directly on your Android or iOS device **without requiring store publishing or a paid Apple Developer Program membership**.

---

## 1. Android: Direct APK Download & Installation

You can download and install the Android APK directly onto any phone or tablet running Android 8.0 (API 26) or higher.

### Step 1: Download the APK
1. Go to the [GitHub Releases](https://github.com/itsvrushabh/rs-budget/releases) page of this repository.
2. Under **Assets** for the latest release, download `actual-budget-mobile-android.apk`.

### Step 2: Enable "Install Unknown Apps"
Because you are downloading the app outside Google Play:
1. Open **Settings** on your Android device.
2. Navigate to **Apps & notifications** (or **Security & Privacy**) > **Special app access** > **Install unknown apps**.
3. Select your browser (e.g. Chrome, Firefox) or File Manager, and toggle **Allow from this source** to **ON**.

### Step 3: Install and Launch
1. Open the downloaded `actual-budget-mobile-android.apk` from your notifications or Downloads folder.
2. Tap **Install**.
3. Tap **Open** to launch Actual Budget.

> [!TIP]
> You can also install directly via USB using Android Debug Bridge (ADB):
> ```bash
> adb install dist/mobile/actual-budget-mobile-android.apk
> ```

---

## 2. iOS & iPadOS: Direct Installation (No Apple Developer Account Required)

Apple restricts running unapproved apps, but you **do not** need a paid $99/year Apple Developer account to run Actual Budget on your iPhone or iPad. You can sideload the unsigned `.ipa` package using a free personal Apple ID.

Supported devices: iPhone, iPad, and iPod touch running iOS / iPadOS 14.0 or newer.

### Method A: AltStore / SideStore (Recommended)

[AltStore](https://altstore.io) allows you to use your free personal Apple ID to sign apps directly on your device.

1. **Install AltServer**: Download and run AltServer on your Mac or Windows PC from [altstore.io](https://altstore.io).
2. **Install AltStore to iPhone/iPad**: Connect your iOS device to your computer via USB or Wi-Fi, click the AltServer tray icon, and choose **Install AltStore > [Your Device]**. Enter your Apple ID and password.
3. **Trust Developer Certificate**:
   - On your iPhone/iPad, go to **Settings > General > VPN & Device Management**.
   - Tap your Apple ID under "Developer App" and tap **Trust**.
4. **Install Actual Budget**:
   - On your device, download `actual-budget-mobile-ios-unsigned.ipa` from [GitHub Releases](https://github.com/itsvrushabh/rs-budget/releases).
   - Open **AltStore**, go to the **My Apps** tab, tap the **+** icon in the top left, and select the downloaded `.ipa`.
   - AltStore will sign and install the app onto your home screen!

> [!NOTE]
> Free Apple IDs permit up to 3 active sideloaded apps at a time, expiring every 7 days. If your computer running AltServer is on the same Wi-Fi network, AltStore automatically refreshes the certificate in the background.

---

### Method B: Sideloadly (Windows & macOS)

[Sideloadly](https://sideloadly.io) provides a one-click drag-and-drop installer:

1. Download and launch **Sideloadly** on your computer.
2. Connect your iPhone or iPad via USB.
3. Download `actual-budget-mobile-ios-unsigned.ipa` on your computer.
4. Drag and drop the `.ipa` file into Sideloadly.
5. Enter your free personal Apple ID email under **Apple ID**.
6. Click **Start**. Sideloadly will fetch signing certificates from Apple and install the app.
7. On your iPhone, go to **Settings > General > VPN & Device Management**, and tap **Trust**.

---

### Method C: TrollStore (iOS 14.0 – 16.6.1 & 17.0)

If your device runs a version of iOS supported by [TrollStore](https://github.com/opa334/TrollStore):
1. Download `actual-budget-mobile-ios-unsigned.ipa`.
2. Tap Share and select **TrollStore**.
3. TrollStore installs the app **permanently** without any 7-day expiration or app limits.

---

### Method D: Xcode Free Provisioning (Mac Developers)

If you have a Mac with Xcode:
1. Open Xcode and go to **Settings > Accounts**.
2. Click **+** and add your free Apple ID (no paid membership required). Xcode creates a free "Personal Team".
3. Open the iOS project in `packaging/ios/` or build the app binary with:
   ```bash
   cargo build --target aarch64-apple-ios --release
   ```
4. Set the bundle identifier to `org.actualbudget.mobile` (or your personal prefix) and select your **Personal Team** under Signing & Capabilities.
5. Select your connected iPhone/iPad as the target and click **Run** (Cmd + R).

---

## 3. Building Mobile Packages Locally

To produce both Android APK and iOS IPA packages on your local workstation:

```bash
# Package both Android and iOS artifacts
./scripts/package-mobile.sh all

# Or package only Android
./scripts/package-mobile.sh android

# Or package only iOS
./scripts/package-mobile.sh ios
```

The output packages will be placed in `dist/mobile/`:
- `actual-budget-mobile-android.apk` (+ `.sha256`)
- `actual-budget-mobile-ios-unsigned.ipa` (+ `.sha256`)
- `README-mobile-install.md`
