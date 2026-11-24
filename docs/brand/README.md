# rmux mark

The violet panes and inset terminal prompt form the rmux mark. `rmux.svg` is the
flat vector master used by Linux and the website. The website keeps a local copy
in its separate repository.

## macOS app icon

`rmux.icon` is the macOS Icon Composer source. It uses a 1024×1024 square canvas,
an opaque solid sRGB background, and a centered vector foreground. The prompt is
a cutout in the foreground rather than a painted background-colored stroke, so
it remains recognizable in dark and tinted appearances. Apple supplies the
outer mask and platform effects; the source has no rounded background, inset
border, or baked shadows. The existing palette and pane symbol are retained.

This follows Apple's current [app icon guidance](https://developer.apple.com/design/human-interface-guidelines/app-icons)
and [Icon Composer workflow](https://developer.apple.com/documentation/xcode/creating-your-app-icon-using-icon-composer).

Regenerate with an **already installed Xcode 26 or later**, selected through
`DEVELOPER_DIR` or the existing `xcode-select` configuration:

```sh
mise exec -- python scripts/make-macos-icon.py
mise exec -- python scripts/make-macos-icon.py --check
```

The compiler creates `macos/Assets.car`, `macos/icon-info.plist`, and `rmux.icns`.
The catalog contains the layered default, dark, and tintable appearances plus
32–1024px flattened representations for older systems. Apple's supplementary
ICNS contains the smaller fallback sizes; it must be shipped **with** the catalog.
The deployment target comes from `release/macos.json`.

These generated files are committed so ordinary builds, including the pinned
release runner, do not need to install or upgrade Apple tooling. The provenance
manifest records compiler version and source/output hashes. Bundling checks it,
installs both resources, and merges Apple's `CFBundleIconName` and
`CFBundleIconFile` keys before code signing. Source edits require regeneration.

To export appearance previews using the existing Icon Composer CLI:

```sh
mise exec -- "$(xcode-select -p)/../Applications/Icon Composer.app/Contents/Executables/ictool" \
  docs/brand/rmux.icon --export-image --output-file target/branding/default.png \
  --platform macOS --rendition Default --width 256 --height 256 --scale 1
```

Use `Dark`, `ClearLight`, `ClearDark`, `TintedLight`, or `TintedDark` for other
appearances. Inspect small sizes as well as the large preview. Live Dock/Finder
appearance and older-system runtime checks remain separate from asset validation.

## Other platforms and marketing

To regenerate the flat Windows artwork using the existing Apple SDK:

```sh
mise exec -- swift scripts/make-app-icon.swift target/branding/rmux.iconset
mise exec -- python scripts/make-windows-icon.py
```

Keep that renderer's geometry and colors in sync with the flat SVG. Do not use
its PNGs to overwrite the macOS compiler output.

Windows embeds `rmux.ico` as resource 1, the icon GPUI loads for its windows.
Linux packages include the SVG and a matching `rmux.desktop` launcher; native
windows use `rmux` as their application ID for desktop grouping.

`github-avatar.png` is the 1024px mark for the organization avatar.
`github-social.png` is the 1280×640 repository link preview, generated with
`mise exec -- swift scripts/make-github-social.swift`. GitHub settings uploads
are handled by the user; these files alone do not change those settings.
