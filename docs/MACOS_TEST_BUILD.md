# macOS development build

This package contains Lan Mouse Dev.app, its GTK dependencies, and the native
edge-range overlay. It is ad-hoc signed, not notarized. It does not replace the
installed application or grant system permissions. Keep your existing app and
configuration backup; quit the existing Lan Mouse before trying this build.

In General, enable **跨 Windows / macOS 自动交换 Alt 与 Super** on both updated
computers for bidirectional swapping. The setting saves immediately and defaults
to off. Only incoming Windows/macOS input swaps Alt/Option and Super/Command;
local and same-platform input stays unchanged. Old peers with unknown platforms
pass through unchanged. Remove legacy `input_pre_processing.remap_keys` entries
and stop separate modifier-swap helpers to avoid conflicting mappings.

Client -> 切换范围 -> 设置 opens the native range selector. Range settings are
still local drafts: they do not change the live crossing boundary or landing point.

The package records its source commit and minimum macOS version in BUILD_INFO.txt.
The minimum is derived from all bundled Mach-O files, including transitive GTK
dependencies and the Swift helper. The current package requires macOS 27.2+;
checking GTK alone is insufficient. This package is Apple Silicon only when built on an ARM Mac.
Rust tests, Clippy, the settings switch preview, native overlay self-tests, bundle
signatures and --version are checked. Two-computer keyboard sharing still needs
acceptance testing on Windows and macOS. Follow normal OS approval flows for
Accessibility/Input Monitoring; do not bypass system security warnings.

Build from the repository root on macOS with Rust 1.91.1, Swift, Homebrew GTK4/
Libadwaita, librsvg, ImageMagick and Python 3 available:

```sh
./tools/build-macos-dev.sh
```

Outputs go under dist/macos. Builds use two jobs and disable LTO to limit memory.
The build script does not install or start the daemon.
