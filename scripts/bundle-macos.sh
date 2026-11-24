#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
profile="${RMUX_BUILD_PROFILE:-debug}"
case "$profile" in
    debug) cargo build -p rmux-desktop -p rmux-cli ;;
    release) cargo build --release -p rmux-desktop -p rmux-cli ;;
    *) printf '%s\n' 'RMUX_BUILD_PROFILE must be debug or release' >&2; exit 1 ;;
esac
bundle="$PWD/target/$profile/rmux.app"
if [ "${RMUX_DISTRIBUTION:-0}" = 1 ]; then
    : "${RMUX_SIGN_IDENTITY:?Set a Developer ID Application signing identity}"
    bundle="$PWD/target/distribution/rmux.app"
fi
mkdir -p "$bundle/Contents/MacOS"
mkdir -p "$bundle/Contents/Resources/shell-integration/zsh"
mkdir -p "$bundle/Contents/Resources/shell-integration/bash"
mkdir -p "$bundle/Contents/Resources/shell-integration/fish"
mkdir -p "$bundle/Contents/Resources/shell-integration/powershell"
cp LICENSE "$bundle/Contents/Resources/LICENSE"
cp shell-integration/powershell/rmux.ps1 "$bundle/Contents/Resources/shell-integration/powershell/rmux.ps1"
cp shell-integration/fish/rmux.fish "$bundle/Contents/Resources/shell-integration/fish/rmux.fish"
cp shell-integration/bash/rmux.bash "$bundle/Contents/Resources/shell-integration/bash/rmux.bash"
cp shell-integration/zsh/rmux.zsh "$bundle/Contents/Resources/shell-integration/zsh/rmux.zsh"
cp "target/$profile/rmux" "$bundle/Contents/MacOS/rmux"
cp "target/$profile/rmuxctl" "$bundle/Contents/MacOS/rmuxctl"
cat > "$bundle/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleExecutable</key><string>rmux</string>
<key>CFBundleIdentifier</key><string>dev.rmux.prototype</string>
<key>CFBundleName</key><string>rmux</string>
<key>CFBundleDisplayName</key><string>rmux</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleVersion</key><string>0.1.0</string>
<key>NSHighResolutionCapable</key><true/>
<key>CFBundleDocumentTypes</key><array><dict>
<key>CFBundleTypeName</key><string>Project Folder</string>
<key>CFBundleTypeRole</key><string>Viewer</string>
<key>LSHandlerRank</key><string>Alternate</string>
<key>LSItemContentTypes</key><array><string>public.folder</string></array>
</dict></array>
</dict></plist>
PLIST
python3 scripts/install-macos-icon.py "$bundle"
if [ "${RMUX_DISTRIBUTION:-0}" = 1 ]; then
    sparkle="$(mise where github:sparkle-project/Sparkle@2.10.0)"
    mkdir -p "$bundle/Contents/Frameworks"
    ditto "$sparkle/Sparkle.framework" "$bundle/Contents/Frameworks/Sparkle.framework"
    cp "$sparkle/LICENSE" "$bundle/Contents/Resources/Sparkle-LICENSE.txt"
    python3 scripts/configure-release.py "$bundle"
    python3 scripts/release-notices.py "$bundle/Contents/Resources/THIRD-PARTY-NOTICES.txt"
    framework="$bundle/Contents/Frameworks/Sparkle.framework"
    for component in "$framework/Versions/B/XPCServices/Downloader.xpc" \
        "$framework/Versions/B/XPCServices/Installer.xpc" \
        "$framework/Versions/B/Autoupdate" "$framework/Versions/B/Updater.app" "$framework"; do
        codesign --force --timestamp --options runtime --sign "$RMUX_SIGN_IDENTITY" "$component"
    done
    codesign --force --timestamp --options runtime --sign "$RMUX_SIGN_IDENTITY" "$bundle/Contents/MacOS/rmuxctl"
    codesign --force --timestamp --options runtime --sign "$RMUX_SIGN_IDENTITY" "$bundle"
    codesign --verify --deep --strict "$bundle"
    printf '%s\n' "$bundle"
    exit 0
fi
# Bind the copied executable, plist and resources into a local development
# signature. The linker's standalone signature is not a complete bundle seal.
codesign --force --sign - "$bundle/Contents/MacOS/rmuxctl"
codesign --force --sign - "$bundle"
printf '%s\n' "$bundle"
