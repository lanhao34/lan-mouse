#!/usr/bin/env bash
# Invoked by build-windows-local.ps1 from the repository root in MSYS2 UCRT64.
set -euo pipefail
export PATH="/ucrt64/bin:$PATH"
export PKG_CONFIG="$(cygpath -m /ucrt64/bin/pkgconf.exe)"
export GSETTINGS_SCHEMA_DIR="$(cygpath -m /ucrt64/share/glib-2.0/schemas)"
export GTK_PREFIX="$(cygpath -m /ucrt64)"
if [[ "${INSTALL_BUILD_DEPS:-0}" == 1 ]]; then
  pacman -S --needed --noconfirm \
    mingw-w64-ucrt-x86_64-rust mingw-w64-ucrt-x86_64-gtk4 \
    mingw-w64-ucrt-x86_64-libadwaita mingw-w64-ucrt-x86_64-librsvg \
    mingw-w64-ucrt-x86_64-pkgconf mingw-w64-ucrt-x86_64-python \
    mingw-w64-ucrt-x86_64-ntldd make
fi
for tool in cargo rustc python ntldd; do
  [[ -x "/ucrt64/bin/$tool.exe" ]] || { echo "Missing UCRT64 $tool; rerun with -InstallBuildDependencies" >&2; exit 2; }
done
/ucrt64/bin/rustc --version
/ucrt64/bin/cargo test --workspace --all-features --locked
/ucrt64/bin/cargo clippy --workspace --all-targets --all-features --locked
/ucrt64/bin/cargo build --release --locked
/ucrt64/bin/cargo build --release -p lan-mouse-gtk --example edge-selector --locked
/ucrt64/bin/python tools/package-windows.py
cp tools/launch-windows.cmd dist/lan-mouse-windows-x64/Launch.cmd
