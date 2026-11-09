#!/bin/sh
# Run from repository root. Builds only the editor, never the daemon.
set -eu
cargo +1.91.1 build -p lan-mouse-gtk --example edge-selector --locked
app_path="target/Edge Range Editor.app"
mkdir -p "$app_path/Contents/MacOS"
cp target/debug/examples/edge-selector "$app_path/Contents/MacOS/edge-selector"
cat > "$app_path/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleIdentifier</key><string>de.feschber.LanMouse.EdgeSelector</string>
<key>CFBundleName</key><string>Edge Range Editor</string>
<key>CFBundleExecutable</key><string>edge-selector</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleVersion</key><string>1</string>
</dict></plist>
PLIST
printf '%s\n' "$app_path"
