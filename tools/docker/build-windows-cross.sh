#!/usr/bin/env bash
# Run from the repository root on a Mac with a running Colima/Docker VM.
# This cross-compiles; it does not run Windows GUI tests or publish a release.
set -euo pipefail
context="${LAN_MOUSE_DOCKER_CONTEXT:-colima-lanmouse-build}"
image="lanmouse-windows-cross:dev"
root="$(pwd)"
test -f "$root/Cargo.toml"
docker --context "$context" image inspect "$image" >/dev/null
mkdir -p "$root/dist/cross-windows"
docker --context "$context" run --rm \
  --mount "type=bind,source=$root,target=/src,readonly" \
  --mount "type=volume,source=lanmouse-windows-cross-target,target=/target" \
  --mount "type=volume,source=lanmouse-windows-cross-cargo,target=/usr/local/cargo/registry" \
  --mount "type=bind,source=$root/dist/cross-windows,target=/dist" \
  -e "HTTP_PROXY=${LAN_MOUSE_BUILD_PROXY:-}" -e "HTTPS_PROXY=${LAN_MOUSE_BUILD_PROXY:-}" \
  -e "http_proxy=${LAN_MOUSE_BUILD_PROXY:-}" -e "https_proxy=${LAN_MOUSE_BUILD_PROXY:-}" \
  -e ALL_PROXY= -e all_proxy= -e NO_PROXY=localhost,127.0.0.1,::1 -e no_proxy=localhost,127.0.0.1,::1 \
  "$image" sh -c '
    set -eu
    git config --global --add safe.directory /src
    cargo build --release --target x86_64-pc-windows-gnu --locked
    cargo build --release --target x86_64-pc-windows-gnu -p lan-mouse-gtk --example edge-selector --locked
    cp /target/x86_64-pc-windows-gnu/release/lan-mouse.exe /dist/
    cp /target/x86_64-pc-windows-gnu/release/examples/edge-selector.exe /dist/edge-settings-preview.exe
    cp /opt/windows/packages.json /dist/windows-libraries.json
    file /dist/*.exe
  '
