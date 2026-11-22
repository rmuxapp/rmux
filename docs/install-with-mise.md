# Install with mise

rmux publishes regular GitHub releases. Mise can install the latest release using
its GitHub backend. Newly published releases may be hidden by mise's default
24-hour minimum release age. To install a new release immediately, pin its exact
version or pass `--minimum-release-age 0s` for that command.

For macOS Apple Silicon, add this to your mise configuration:

```toml
[tools."github:rmuxapp/rmux"]
version = "latest"
asset_pattern = "rmux-*-macos-arm64.zip"
strip_components = 0
bin_path = "rmux.app/Contents/MacOS"
```

Run `mise install`, then `mise exec -- rmux` or `mise exec -- rmuxctl --help`.
Preserving the `.app` directory keeps its resources and framework paths together.
For Finder installation and Sparkle-managed updates, prefer the DMG instead;
use mise to manage versions of a mise installation.

On Linux x86-64, use the same version setting with
`asset_pattern = "rmux-*-linux-x86_64.tar.gz"`. On Windows x86-64 use
`asset_pattern = "rmux-*-windows-x86_64.zip"`. Omit `strip_components` and
`bin_path` on those platforms so mise detects the archive's top-level directory.

For example, to resolve a newly published regular release immediately:

```sh
mise use --minimum-release-age 0s github:rmuxapp/rmux@latest
```

Keep the platform-specific archive options above. Add `prerelease = true` only
if you explicitly want to include any future prereleases.

The macOS archive was installed successfully in isolated mise directories,
including the preserved .app layout, and rmuxctl executed successfully.
Native Windows/Linux mise execution and macOS GUI launch acceptance are pending.
See the [mise GitHub backend documentation](https://mise.jdx.dev/dev-tools/backends/github.html).
