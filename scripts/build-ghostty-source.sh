#!/bin/sh
# Project-local library build; Zig comes from the pinned mise environment.
set -eu
cd "$(dirname "$0")/.."
root="$PWD"
version=1.0.1
archive="$root/.deps/libghostty-vt-$version.tar.gz"
expected=333dfce97641ef9b536bb9e08907ad86abaaada50e8f7417b70d9284adef37aa
mkdir -p .deps
if [ ! -f "$archive" ]; then
    curl --fail --location --retry 2 "https://release.files.ghostty.org/libghostty-vt/$version/libghostty-vt-$version.tar.gz" -o "$archive.part"
    mv "$archive.part" "$archive"
fi
if command -v sha256sum >/dev/null 2>&1; then
    printf '%s  %s\n' "$expected" "$archive" | sha256sum -c -
else
    printf '%s  %s\n' "$expected" "$archive" | shasum -a 256 -c -
fi
if [ ! -d ".deps/libghostty-vt-$version" ]; then tar -xzf "$archive" -C .deps; fi
test "$(zig version)" = '0.17.0' || { echo 'Run through mise with pinned Zig 0.17.0' >&2; exit 1; }
cd ".deps/libghostty-vt-$version"
# Optional target builds are isolated from the native installation, preventing
# a cross-compiled archive from silently replacing the host library.
if [ -n "${RMUX_GHOSTTY_TARGET:-}" ]; then
    case "$RMUX_GHOSTTY_TARGET" in
        wasm32-freestanding)
            zig build -Demit-lib-vt -Dtarget=wasm32-freestanding -Doptimize=ReleaseSmall --prefix "$root/.deps/ghostty-vt-wasm32-freestanding"
            exit 0
            ;;
        x86_64-linux-gnu|aarch64-linux-gnu|x86_64-windows-gnu|aarch64-windows-gnu) ;;
        *) echo 'Unsupported cross-build target' >&2; exit 1 ;;
    esac
    zig build -Doptimize=ReleaseFast "-Dtarget=$RMUX_GHOSTTY_TARGET" --prefix "$root/.deps/ghostty-vt-$RMUX_GHOSTTY_TARGET"
else
    zig build -Doptimize=ReleaseFast --prefix "$root/.deps/ghostty-vt"
fi
