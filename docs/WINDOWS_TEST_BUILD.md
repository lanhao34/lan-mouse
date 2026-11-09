# Windows x64 development build

This is an unsigned development build for Windows 10/11 x64, not a stable release.
Extract the entire ZIP into a new directory. Keep the DLLs, share directory and
native-edge-overlay directory together. Do not copy only lan-mouse.exe.

- `Launch.cmd preview`: offline client-settings preview; does not connect to the running daemon.
- `Launch.cmd`: full Lan Mouse application with outgoing key remapping and the new range settings page.
- Expand a client -> 切换范围 -> 设置 -> 在屏幕上调整.
- Windows uses native small topmost windows: a click-through line, two handles,
  and a small confirm/cancel panel. There is no fullscreen desktop-sized window.
- Cancel closes the overlay without saving. Escape works with the panel focused.
  Display layout changes and a five-minute editing timeout cancel the session.
- Mixed DPI/negative monitor origins are matched before selection. An ambiguous
  match is rejected. Confirmed range settings are local drafts only: they do NOT
  yet change the live crossing boundary or cursor landing point.

Do not run two Lan Mouse daemons or combine native Alt/Meta swapping with a second
modifier-swapping tool on the receiver. Back up your existing configuration and
keep the old installation when trying a new build. This package does not alter
firewall rules, elevate privileges, install drivers, or add startup tasks.

The helper includes its .NET runtime. The main app includes GTK/UCRT dependencies.
Launch.cmd sets portable GTK paths and regenerates image-loader metadata locally.
The source revision is in SOURCE_COMMIT.txt; runtime package versions are recorded
in RUNTIME_PACKAGES.txt. Source is available at https://github.com/lanhao34/lan-mouse.

Automated validation covers Rust tests, native helper geometry tests, native helper
window lifecycle smoke testing, and loading the packaged executable with --version.
Actual two-computer key sharing and real mixed-DPI pointer interaction still require
manual acceptance testing. Do not bypass Windows security warnings to run this build.
