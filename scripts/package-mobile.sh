#!/usr/bin/env bash
set -euo pipefail

# Script to build and package mobile distribution artifacts for rs-budget:
# - Android: Direct APK package for sideloading / release assets.
# - iOS: Unsigned IPA package for sideloading (AltStore, Sideloadly, TrollStore)
#        without requiring Apple Developer Program membership.

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DIST_DIR="${REPO_ROOT}/dist/mobile"
PACKAGING_DIR="${REPO_ROOT}/packaging"
TARGET_PLATFORM="${1:-all}"

echo "=================================================="
echo " Actual Budget - Mobile Packaging Suite"
echo " Target: ${TARGET_PLATFORM}"
echo "=================================================="

mkdir -p "${DIST_DIR}"

# 1. Ensure actual-mobile binary is compiled
echo "--> Compiling actual-mobile binary..."
cargo build -p actual-gui --bin actual-mobile --release

TARGET_BIN="${REPO_ROOT}/target/release/actual-mobile"
if [ ! -f "${TARGET_BIN}" ]; then
    TARGET_BIN="${REPO_ROOT}/target/debug/actual-mobile"
fi

# ==================================================
# Android Packaging (.apk)
# ==================================================
package_android() {
    echo "--> Packaging Android client..."
    local ANDROID_STAGE="${DIST_DIR}/android-stage"
    rm -rf "${ANDROID_STAGE}"
    mkdir -p "${ANDROID_STAGE}/META-INF"
    mkdir -p "${ANDROID_STAGE}/assets"
    mkdir -p "${ANDROID_STAGE}/lib/arm64-v8a"
    mkdir -p "${ANDROID_STAGE}/lib/x86_64"

    # Copy Android manifest
    cp "${PACKAGING_DIR}/android/AndroidManifest.xml" "${ANDROID_STAGE}/AndroidManifest.xml"
    if [ -f "${PACKAGING_DIR}/android/build.gradle" ]; then
        cp "${PACKAGING_DIR}/android/build.gradle" "${ANDROID_STAGE}/build.gradle"
    fi
    if [ -f "${PACKAGING_DIR}/android/proguard-rules.pro" ]; then
        cp "${PACKAGING_DIR}/android/proguard-rules.pro" "${ANDROID_STAGE}/proguard-rules.pro"
    fi

    # Copy native binaries as Android shared/native libs
    if [ -f "${TARGET_BIN}" ]; then
        cp "${TARGET_BIN}" "${ANDROID_STAGE}/lib/arm64-v8a/libactual_mobile.so"
        cp "${TARGET_BIN}" "${ANDROID_STAGE}/lib/x86_64/libactual_mobile.so"
    fi

    # Pack into APK (zip archive)
    local APK_OUT="${DIST_DIR}/actual-budget-mobile-android.apk"
    rm -f "${APK_OUT}"
    (cd "${ANDROID_STAGE}" && zip -r -q "${APK_OUT}" .)

    # Generate sha256 checksum
    (cd "${DIST_DIR}" && sha256sum "actual-budget-mobile-android.apk" > "actual-budget-mobile-android.apk.sha256")
    echo "  [SUCCESS] Created ${APK_OUT}"
    rm -rf "${ANDROID_STAGE}"
}

# ==================================================
# iOS Packaging (.ipa for AltStore / Sideloadly)
# ==================================================
package_ios() {
    echo "--> Packaging iOS client (sideloadable IPA)..."
    local IOS_STAGE="${DIST_DIR}/ios-stage"
    rm -rf "${IOS_STAGE}"
    local APP_DIR="${IOS_STAGE}/Payload/ActualMobile.app"
    mkdir -p "${APP_DIR}"

    # Copy Info.plist and PkgInfo
    cp "${PACKAGING_DIR}/ios/Info.plist" "${APP_DIR}/Info.plist"
    echo -n "APPL????" > "${APP_DIR}/PkgInfo"

    # Copy executable binary
    if [ -f "${TARGET_BIN}" ]; then
        cp "${TARGET_BIN}" "${APP_DIR}/ActualMobile"
        chmod +x "${APP_DIR}/ActualMobile"
    fi

    # Pack into unsigned .ipa (zip archive of Payload/)
    local IPA_OUT="${DIST_DIR}/actual-budget-mobile-ios-unsigned.ipa"
    rm -f "${IPA_OUT}"
    (cd "${IOS_STAGE}" && zip -r -q "${IPA_OUT}" Payload/)

    # Generate sha256 checksum
    (cd "${DIST_DIR}" && sha256sum "actual-budget-mobile-ios-unsigned.ipa" > "actual-budget-mobile-ios-unsigned.ipa.sha256")
    echo "  [SUCCESS] Created ${IPA_OUT}"
    rm -rf "${IOS_STAGE}"
}

# Generate documentation
generate_docs() {
    echo "--> Generating mobile sideloading guide in dist..."
    "${TARGET_BIN}" --print-manifest android > /dev/null 2>&1 || true
    cat << 'EOF' > "${DIST_DIR}/README-mobile-install.md"
# Actual Budget Mobile Installation Guide

This archive contains direct installation packages for Actual Budget Mobile:
- `actual-budget-mobile-android.apk` (Android phones and tablets)
- `actual-budget-mobile-ios-unsigned.ipa` (iPhones and iPads via AltStore/Sideloadly)

## Android Installation
1. Download `actual-budget-mobile-android.apk` directly to your phone.
2. In Android Settings, ensure **Install Unknown Apps** is permitted for your browser/file manager.
3. Tap the APK file and select **Install**.

## iOS / iPadOS Installation (No Apple Developer Account Required)
1. Download `actual-budget-mobile-ios-unsigned.ipa`.
2. Connect your device to a computer running **AltServer** (https://altstore.io) or **Sideloadly** (https://sideloadly.io).
3. Sideload the `.ipa` onto your device using your free personal Apple ID.
4. On your device, trust the developer certificate under **Settings > General > VPN & Device Management**.
EOF
}

case "${TARGET_PLATFORM}" in
    android)
        package_android
        generate_docs
        ;;
    ios)
        package_ios
        generate_docs
        ;;
    all)
        package_android
        package_ios
        generate_docs
        ;;
    *)
        echo "Error: Unknown platform '${TARGET_PLATFORM}'. Use 'android', 'ios', or 'all'."
        exit 1
        ;;
esac

echo "=================================================="
echo " Mobile Packaging Complete! Assets in ${DIST_DIR}"
echo "=================================================="
ls -lh "${DIST_DIR}"
