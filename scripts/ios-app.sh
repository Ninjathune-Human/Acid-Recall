#!/usr/bin/env bash
# Builds ACID RECALL for iPad and wraps it into an app bundle (docs/ipad.md):
#
#   scripts/ios-app.sh sim      # the Simulator of this Mac: target/ios/sim/AcidRecall.app
#                               # (aarch64-apple-ios-sim on Apple silicon, x86_64-apple-ios
#                               # on an Intel Mac)
#   scripts/ios-app.sh device   # aarch64-apple-ios: target/ios/device/AcidRecall.app and
#                               # target/ios/device/AcidRecall.ipa
#
# The bundle gets the UI binary (CFBundleExecutable AcidRecall), the app icon at the two iPad
# sizes and an Info.plist for an iPad in landscape, full screen, with its Documents folder shown
# in the Files app (songs and samples go there). It is signed ad hoc: the Simulator on Apple
# silicon refuses unsigned code. The .ipa is not signed for a device: a sideloading tool
# (Sideloadly, AltStore) signs it with the user's Apple ID when it installs it.
#
# Environment: SOURCE_DATE_EPOCH, BUNDLE_ID (default io.github.ninjathune-human.acidrecall),
# IPHONEOS_DEPLOYMENT_TARGET (default 16.0).
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
usage="usage: scripts/ios-app.sh sim|device"
case ${1:-} in
    sim)
        platform=iPhoneSimulator
        if [ "$(uname -m)" = arm64 ]; then target=aarch64-apple-ios-sim; else target=x86_64-apple-ios; fi
        ;;
    device) target=aarch64-apple-ios; platform=iPhoneOS ;;
    *) echo "$usage" >&2; exit 2 ;;
esac
kind=$1

[ "$(uname -s)" = Darwin ] || { echo "macOS only (Xcode, sips, codesign)" >&2; exit 1; }

export IPHONEOS_DEPLOYMENT_TARGET=${IPHONEOS_DEPLOYMENT_TARGET:-16.0}
export SOURCE_DATE_EPOCH=${SOURCE_DATE_EPOCH:-$(date +%s)}
version=$("$root/scripts/version.sh")   # 2026.1117.1030
bundle_id=${BUNDLE_ID:-io.github.ninjathune-human.acidrecall}

cargo build --manifest-path "$root/Cargo.toml" --release --locked --target "$target" \
    -p orb-ui --bin OpenRB-338
bin=${CARGO_TARGET_DIR:-$root/target}/$target/release/OpenRB-338

out=$root/target/ios/$kind
app=$out/AcidRecall.app
rm -rf "$out"
mkdir -p "$app"
cp -p "$bin" "$app/AcidRecall"
chmod 755 "$app/AcidRecall"

# The icon: the brand PNG at the iPad sizes (76 pt and 83.5 pt at 2x).
icon=$root/assets/brand/acid-recall.png
sips -z 152 152 "$icon" --out "$app/AppIcon76x76@2x~ipad.png" >/dev/null
sips -z 167 167 "$icon" --out "$app/AppIcon83.5x83.5@2x~ipad.png" >/dev/null

cat > "$app/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleExecutable</key>
    <string>AcidRecall</string>
    <key>CFBundleIdentifier</key>
    <string>$bundle_id</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>ACID RECALL</string>
    <key>CFBundleDisplayName</key>
    <string>ACID RECALL</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>$version</string>
    <key>CFBundleVersion</key>
    <string>$version</string>
    <key>CFBundleSupportedPlatforms</key>
    <array>
        <string>$platform</string>
    </array>
    <key>CFBundleIcons~ipad</key>
    <dict>
        <key>CFBundlePrimaryIcon</key>
        <dict>
            <key>CFBundleIconFiles</key>
            <array>
                <string>AppIcon76x76</string>
                <string>AppIcon83.5x83.5</string>
            </array>
        </dict>
    </dict>
    <key>MinimumOSVersion</key>
    <string>$IPHONEOS_DEPLOYMENT_TARGET</string>
    <key>LSRequiresIPhoneOS</key>
    <true/>
    <key>UIDeviceFamily</key>
    <array>
        <integer>2</integer>
    </array>
    <key>UIRequiresFullScreen</key>
    <true/>
    <key>UILaunchScreen</key>
    <dict/>
    <key>UIStatusBarHidden</key>
    <true/>
    <key>UIViewControllerBasedStatusBarAppearance</key>
    <false/>
    <key>UISupportedInterfaceOrientations</key>
    <array>
        <string>UIInterfaceOrientationLandscapeLeft</string>
        <string>UIInterfaceOrientationLandscapeRight</string>
    </array>
    <key>UISupportedInterfaceOrientations~ipad</key>
    <array>
        <string>UIInterfaceOrientationLandscapeLeft</string>
        <string>UIInterfaceOrientationLandscapeRight</string>
    </array>
    <key>UIFileSharingEnabled</key>
    <true/>
    <key>LSSupportsOpeningDocumentsInPlace</key>
    <true/>
    <key>NSHumanReadableCopyright</key>
    <string>Copyright (C) 2026 the OpenRB-338 authors. GPL-3.0-only.</string>
</dict>
</plist>
EOF
plutil -lint "$app/Info.plist" >/dev/null

codesign --force --sign - "$app"
codesign --verify --strict "$app"

if [ "$kind" = device ]; then
    # An .ipa is a zip of Payload/<name>.app.
    rm -rf "$out/Payload"
    mkdir "$out/Payload"
    cp -R "$app" "$out/Payload/"
    (cd "$out" && zip -qry AcidRecall.ipa Payload)
    rm -rf "$out/Payload"
    echo "$out/AcidRecall.ipa"
else
    echo "$app"
fi
