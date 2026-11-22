# Tool management

Manage all development tooling through mise. Declare pinned tool versions in
`mise.toml` and run project commands through `mise exec` or mise tasks.

Do not install tools through Homebrew, direct rustup commands, standalone
installers, or other mechanisms without explicit user approval. Tool installs
managed by mise are authorized. Do not create `rust-toolchain.toml` files that
trigger independent rustup auto-installation.

Preserve pre-existing user toolchains and system tools when changing project
setup.

# Delivery focus and iteration cadence

Prioritize polish and release quality of existing workflows over expanding the
feature inventory. Keep the full parity roadmap as longer-term scope.

Batch related implementation changes before validation. Use focused checks for
the affected behavior and one combined build at a meaningful batch boundary;
do not rebuild, run broad suites, push, or deploy after every small edit.
Repeat checks only when subsequent changes or failures justify them. Release
builds run on GitHub. Keep native Windows runtime validation deferred unless
the user asks to resume it.
