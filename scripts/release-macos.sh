#!/bin/sh
# Run via mise run release-macos. Credentials never enter the repository.
set -eu
cd "$(dirname "$0")/.."
: "${RMUX_SIGN_IDENTITY:?Set your Developer ID Application certificate identity}"
: "${RMUX_NOTARY_PROFILE:?Set an existing notarytool Keychain profile name}"
export RMUX_BUILD_PROFILE=release RMUX_DISTRIBUTION=1
sh scripts/bundle-macos.sh
version="$(python3 -c 'import tomllib; print(tomllib.load(open("Cargo.toml", "rb"))["workspace"]["package"]["version"])')"
app="$PWD/target/distribution/rmux.app"
output="$PWD/target/releases/$version"
mkdir -p "$output"
archive="$output/rmux-$version-macos-arm64.zip"
[ "$(uname -m)" = arm64 ] || { printf '%s\n' 'This initial release task currently requires an Apple Silicon host.' >&2; exit 1; }
ditto -c -k --sequesterRsrc --keepParent "$app" "$archive"
if [ -n "${RMUX_NOTARY_KEYCHAIN:-}" ]; then
    xcrun notarytool submit "$archive" --keychain-profile "$RMUX_NOTARY_PROFILE" --keychain "$RMUX_NOTARY_KEYCHAIN" --wait --output-format json > "$output/notarization.json"
else
    xcrun notarytool submit "$archive" --keychain-profile "$RMUX_NOTARY_PROFILE" --wait --output-format json > "$output/notarization.json"
fi
python3 -c 'import json,sys; r=json.load(open(sys.argv[1])); print("Notarization:",r["status"],r["id"]); sys.exit(0 if r["status"]=="Accepted" else 1)' "$output/notarization.json"
xcrun stapler staple "$app"
xcrun stapler validate "$app"
spctl --assess --type execute --verbose=2 "$app"
ditto -c -k --sequesterRsrc --keepParent "$app" "$archive"
set -- --account rmux
if [ -n "${RMUX_SPARKLE_KEY_FILE:-}" ]; then
    set -- --ed-key-file "$RMUX_SPARKLE_KEY_FILE"
fi
cp "docs/releases/v$version.md" "$output/rmux-$version-macos-arm64.md"
generate_appcast "$@" --maximum-deltas 0 --embed-release-notes \
    --full-release-notes-url "https://github.com/rmuxapp/rmux/releases/tag/v$version" \
    --download-url-prefix "https://github.com/rmuxapp/rmux/releases/download/v$version/" \
    --link "https://github.com/rmuxapp/rmux" "$output"
# Keep Sparkle's feed pointed at the signed ZIP; create the installer afterward.
staging="$(mktemp -d)"
trap 'rm -rf "$staging"' EXIT HUP INT TERM
ditto "$app" "$staging/rmux.app"
ln -s /Applications "$staging/Applications"
# Persist icon view and left-to-right installation order on every fresh mount.
cp assets/macos/dmg-layout.dsstore "$staging/.DS_Store"
dmg="$output/rmux-$version-macos-arm64.dmg"
hdiutil create -volname rmux -srcfolder "$staging" -format UDZO -fs HFS+ "$dmg"
codesign --force --timestamp --sign "$RMUX_SIGN_IDENTITY" "$dmg"
if [ -n "${RMUX_NOTARY_KEYCHAIN:-}" ]; then
    xcrun notarytool submit "$dmg" --keychain-profile "$RMUX_NOTARY_PROFILE" --keychain "$RMUX_NOTARY_KEYCHAIN" --wait --output-format json > "$output/notarization-dmg.json"
else
    xcrun notarytool submit "$dmg" --keychain-profile "$RMUX_NOTARY_PROFILE" --wait --output-format json > "$output/notarization-dmg.json"
fi
python3 -c 'import json,sys; r=json.load(open(sys.argv[1])); print("DMG notarization:",r["status"],r["id"]); sys.exit(0 if r["status"]=="Accepted" else 1)' "$output/notarization-dmg.json"
xcrun stapler staple "$dmg"
xcrun stapler validate "$dmg"
(cd "$output" && shasum -a 256 "rmux-$version-macos-arm64.zip" "rmux-$version-macos-arm64.dmg" > SHA256SUMS)
printf '%s\n' "Release artifacts: $output"
