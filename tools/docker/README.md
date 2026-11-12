# Mac Docker → Windows build environment

This is a Linux ARM64 container with a MinGW x64 compiler, Rust 1.91.1's Windows
GNU target and official MSYS2 UCRT64 GTK libraries. It builds Windows PE files;
it is not a Windows VM and cannot certify Windows GUI behavior.

The current setup uses a dedicated Colima profile, `lanmouse-build` (4 CPU,
4 GiB RAM, 24 GiB virtual data disk). It does not use or reset the default profile.
No paid runner, Windows installer, system firewall change or login startup is used.

From the repository root, with this profile running:

```sh
colima --profile lanmouse-build start --cpu 4 --memory 4 --disk 24 --vm-type vz --activate=false
# Build the image inside the VM if host BuildKit forwarding is unreliable:
colima --profile lanmouse-build ssh -- docker build \
  -f "$PWD/tools/docker/Dockerfile.windows-cross" \
  -t lanmouse-windows-cross:dev "$PWD/tools/docker"
./tools/docker/build-windows-cross.sh
```

If the host requires an HTTP proxy, Docker daemon proxy settings inside this
specific VM and build-time HTTP_PROXY/HTTPS_PROXY arguments may be needed. The
proxy address must be reachable from the VM/container; host `127.0.0.1` is not the
container's host. Set `LAN_MOUSE_BUILD_PROXY` to that address for the build script.
Do not disable TLS validation. A missing VM resolver must be corrected to a valid
resolver for the current network, without changing the host's network settings.

The source is mounted read-only. Compiler outputs and Cargo downloads use dedicated
named Docker volumes; only final binaries/manifests go under `dist/cross-windows`.
LTO is disabled for this development build to limit memory use. Rustfmt/Clippy and
native Mac tests remain separate from Windows execution testing.

The library fetcher follows the official repository dependency metadata and checks
each archive SHA256 before extraction, recording exact versions in packages.json.
The downloaded library snapshot can change when rebuilding the image; preserve the
manifest with artifacts. This is not the same as a reproducible pinned sysroot.

Stop the VM when finished with `colima --profile lanmouse-build stop`. Do not delete
its volumes if you want to preserve the toolchain and compilation cache. Building
.exe files alone is not permission to replace the user's installed applications.

## Verified setup

The dedicated Colima VM and image were brought up on an Apple Silicon Mac. A Rust
smoke program cross-linked into a PE32+ x86-64 Windows executable. The application
build exposed a pre-existing bug: three build scripts selected Unix backends using
HOST `cfg!` values. They now use `CARGO_CFG_TARGET_FAMILY`/`CARGO_CFG_TARGET_OS`.
`test-target-cfg.py` checks the capture and emulation build scripts for Windows,
macOS and Linux targets; the full Windows cross-build exercises the root script.

The downloaded sysroot currently contains 83 official UCRT64 packages (including
GTK 4.24.1 and Libadwaita 1.10.0). Rust and native MinGW code are compiled by the
Debian GNU cross toolchain, while GTK DLLs come from UCRT64; Windows runtime tests
remain necessary before treating the resulting package as a supported release.
