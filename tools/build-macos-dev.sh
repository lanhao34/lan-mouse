#!/usr/bin/env bash
# Run from the repository root. Package only; never install or start the daemon.
set -euo pipefail
test "$(uname -s)" = Darwin
test -f Cargo.toml
revision="$(git rev-parse HEAD)"
arch="$(uname -m)"
stage="dist/macos/stage-${revision:0:8}-$arch"
app="$stage/Lan Mouse Dev.app"
test ! -e "$stage" || { echo "Preserve or move the existing stage first: $stage" >&2; exit 1; }
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"
export CARGO_PROFILE_RELEASE_LTO=false
export CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16
cargo +1.91.1 build --release --locked
scripts/makeicns.sh
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp target/release/lan-mouse "$app/Contents/MacOS/lan-mouse"
cp target/icon.icns target/menubar-template.png "$app/Contents/Resources/"
python3 - "$app" "$revision" <<'PY'
import plistlib, sys
from pathlib import Path
app = Path(sys.argv[1])
info = dict(CFBundleIdentifier='de.feschber.LanMouse', CFBundleName='Lan Mouse Dev',
    CFBundleExecutable='lan-mouse', CFBundlePackageType='APPL', CFBundleVersion='0.11.0',
    CFBundleShortVersionString='0.11.0', CFBundleIconFile='icon.icns',
    LSUIElement=True, NSAppSleepDisabled=True, NSHighResolutionCapable=True,
    SourceCommit=sys.argv[2],
    NSInputMonitoringUsageDescription='Lan Mouse captures keyboard and mouse input for authorized remote computers.',
    NSAppleEventsUsageDescription='Lan Mouse delivers synthesized input to this computer.')
(app/'Contents/Info.plist').write_bytes(plistlib.dumps(info))
PY
helper="$app/Contents/Helpers/EdgeOverlay.app"
mkdir -p "$helper/Contents/MacOS"
swiftc lan-mouse-gtk/native/edge-overlay.swift -o "$helper/Contents/MacOS/edge-overlay"
python3 - "$helper" <<'PY'
import plistlib, sys
from pathlib import Path
info = dict(CFBundleIdentifier='de.feschber.LanMouse.EdgeOverlay',
    CFBundleName='Edge Overlay', CFBundleExecutable='edge-overlay',
    CFBundlePackageType='APPL', CFBundleVersion='1', LSUIElement=True)
(Path(sys.argv[1])/'Contents/Info.plist').write_bytes(plistlib.dumps(info))
PY
"$helper/Contents/MacOS/edge-overlay" --self-test
codesign --force --sign - "$helper"
scripts/copy-macos-dylib.sh "$app/Contents/MacOS/lan-mouse"
python3 - "$app" "$stage" "$revision" "$arch" <<'PY'
import plistlib, re, subprocess, sys
from pathlib import Path
app, stage = map(Path, sys.argv[1:3])
versions = []
for binary in [app/'Contents/MacOS/lan-mouse', app/'Contents/Helpers/EdgeOverlay.app/Contents/MacOS/edge-overlay', *list((app/'Contents/Frameworks').iterdir())]:
    data = subprocess.check_output(['otool', '-l', str(binary)], text=True)
    versions.extend(tuple(map(int, v.split('.'))) for v in re.findall(r'\bminos\s+([\d.]+)', data))
    versions.extend(tuple(map(int, v.split('.'))) for v in re.findall(r'LC_VERSION_MIN_MACOSX\s+cmdsize\s+\d+\s+version\s+([\d.]+)', data))
minimum = '.'.join(map(str, max(versions)))
plist = app/'Contents/Info.plist'
info = plistlib.loads(plist.read_bytes())
info['LSMinimumSystemVersion'] = minimum
plist.write_bytes(plistlib.dumps(info))
(stage/'BUILD_INFO.txt').write_text(f'Source commit: {sys.argv[3]}\nArchitecture: {sys.argv[4]}\nMinimum macOS: {minimum}\nAd-hoc signed; not notarized.\n')
PY
codesign --force --sign - "$app"
codesign --verify --deep --strict --verbose=2 "$app"
"$app/Contents/MacOS/lan-mouse" --version
cp docs/MACOS_TEST_BUILD.md LICENSE "$stage/"
archive="dist/macos/lan-mouse-macos-$arch-${revision:0:8}.zip"
ditto -c -k --sequesterRsrc "$stage" "$archive"
python3 - "$archive" <<'PY'
import hashlib, sys
from pathlib import Path
p = Path(sys.argv[1])
sha = hashlib.sha256(p.read_bytes()).hexdigest()
p.with_suffix('.zip.sha256').write_text(f'{sha}  {p.name}\n')
print(p, p.stat().st_size, 'bytes', sha)
PY
