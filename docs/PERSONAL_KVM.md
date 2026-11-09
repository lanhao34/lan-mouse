# Personal KVM development

## Current implementation status

- Base: upstream `69958c625163649b69e27d0a6fb5a3edd596a467`.
- Merged upstream PR #538 at `6b3c894d0801c88790180402035f1e94e760400b`, preserving authorship. It provides global outgoing key remapping and scroll inversion, with matching modifier-mask and key-release handling. For Windows keyboard -> Mac, the Windows sender must run/configure this implementation. Do not simultaneously apply the existing Mac modifier-swapping helper, or a double swap may cancel the desired behavior.
- PR #483 is the broader original remapping work. Do not merge both copies of the feature.
- PR #494 at `21297b089badcbb0e49af6a4819e534a9ca9c2b5` is fetched as `review/edge-position`, NOT merged. Its screen geometry suite was exercised separately (25 tests). It adds negotiated positioned transitions, transition IDs and retry handling. Mac capture and emulation positions are explicitly not implemented in that PR.
- `input-event::edge_mapping` is a new, tested geometry API: monotonic local calibration anchors, normalized positions, flipped segments, strict validation and out-of-range rejection. It is NOT wired into capture, configuration, protocol, or UI yet. No running application has been replaced.

## Intended user experience

Each computer owns only its local portal definition: monitor, edge, start/end points and optional calibration anchors. The user chooses a peer and matching portal identifier, rather than calculating remote desktop coordinates. A pairing preview shows both portals and verifies orientation.

For a source local position x between a and b, t=(x-a)/(b-a). The destination uses its own c,d to place the pointer at c+t*(d-c). Optional monotonic anchors allow alignment points to correspond exactly. Physical screen dimensions and top/center/bottom or manual alignment generate these local anchors; dimensions must not be guessed from pixels or unreliable EDID alone.

Use logical desktop coordinates at each OS boundary; scale/physical units belong to setup and preview. Keep display identity separate from transient screen index. Resolve coordinates from current monitor geometry, including negative origins, scaling and rotation. When a configured monitor disappears, disable that portal rather than silently substituting another.

The destination positions the cursor just inside its configured edge and creates the return trigger on THAT SAME portal. A short exit/entry hysteresis avoids immediate ping-pong. Internal borders between local displays must not trigger remote transitions.

## Implementation sequence

1. Validate the remapping merge on Mac; build/test Windows separately before deployment. Add per-peer mappings if multiple outgoing systems need different layouts.
2. Integrate/review PR #494 on a separate checkpoint. Supply Mac crossing positions and Mac destination warps using CoreGraphics. Test existing old-peer fallback, lost/repeated Enter/Ack, two-way return, multi-monitor gaps and scaled displays.
3. Wire local portal configuration and `EdgeMapping` into both Mac and Windows capture/emulation. Negotiate a portal protocol capability. A transition identifies paired portal, fraction, configuration revision and transition identity. Do not send new messages without peer capability agreement; a peer lacking custom portals must not silently use an unrelated full edge.
4. Add GTK monitor layout editing: detected monitors, optional physical width/height, alignment presets, draggable endpoints and monotonic anchor points. Preserve existing full-edge behavior as the default for unconfigured peers.
5. Add a local edit-only overlay: highlight active edge intervals, show endpoint/anchor handles and live peer landing preview. No input interception during normal use. Escape/cancel restores the prior configuration; apply validates overlaps and records one configuration revision atomically.

## Validation and safety

Tests must cover negative origins, unequal DPI/resolutions, flipped ports, gaps/internal edges, exact endpoints, resizing/disconnection, malformed wire fractions, monotonic anchor ordering, bidirectional round trips, lost/duplicate/stale transitions and keys held while changing configuration.

Do not alter live Lan Mouse config, certificates, firewall, TCC permissions or the running helper while developing. Build output alone is not an end-to-end result. Real Mac/Windows crossing and held-key tests require a controlled release and rollback to the current installed applications.

## Local validation (2026-11-08)

Installed Rust 1.91.1 side-by-side (the existing default toolchain is unchanged).
Use `cargo +1.91.1` in this checkout.

- `cargo +1.91.1 test --no-default-features --locked`: 18 tests passed (existing daemon-only dead-code warning).
- `cargo +1.91.1 test -p input-event --locked`: 3 new edge-mapping tests passed.
- PR #494 geometry file, compiled as an isolated Rust test module: 25 tests passed.
- `cargo +1.91.1 fmt --all --check`: passed.
- `cargo +1.91.1 clippy --workspace --all-targets --all-features --locked`: passed on this Mac, including GTK compilation/checking.
- `git diff --check`: passed.
- No Windows build, physical crossing test, overlay, or live deployment was performed.

## Range editor milestone (2026-11-09)

Added a native GTK editor and an independent example app (no daemon/IPC connection).
Primary selection now occurs on a fullscreen surface on the selected real monitor,
not in a miniature diagram. Click on the actual edge to position its nearest
endpoint or drag a handle; confirm copies the interval into the draft, Escape
cancels. The overlay may obscure underlying desktop content depending on platform
compositing; it still occupies the actual chosen display. No global event taps,
recording permissions, certificate changes or live input settings are involved.

Review fixes: validate imported schema/ranges/finite values, reject ambiguous
monitor snapshots, bound import reads to 64 KiB, revalidate monitor geometry before
export, tolerate floating-point representation at the 0.1% minimum interval, and
break editor signal-reference cycles when closing. Group click/drag gestures so
both can operate on the same boundary surface.

Tests: 7 input-event and 5 GTK-library tests passed; workspace all-target/all-feature
Clippy and formatting passed. Native UI verification confirmed monitor enumeration,
real-screen overlay placement, edge click -> 26.9% start selection, confirmation
back to the editor, and Escape cancellation. Drag motion and actual file chooser
save/import are not fully verified end-to-end. The JSON serialization round trip is
unit tested. This does not certify the running input-sharing pipeline or PR #494.

Remaining: protocol/backend integration (including CoreGraphics vs GDK coordinate
conversion), stable monitor identity, per-monitor exposed-edge validation, editable
physical sizes and multi-point calibration UI, persistent always-visible markers,
and Windows real-display tests. The draft's selected range alone does not change
where the cursor currently switches computers.

## macOS native overlay correction (2026-11-09)

Replaced the macOS fullscreen editing surface with an AppKit helper: one 4-point
click-through line, two 24-point borderless handle panels, and a 420-point-wide
control panel. No desktop-sized input surface is created; panels join existing
Spaces rather than entering fullscreen. GTK editor hides while selection is active
and restores afterward. Dragging uses deltas from the initial range to avoid a jump
when pressing the edge-clamped handle. No global keyboard/mouse monitors or new
permissions are requested. Cancel is always available in the panel; Escape works
while it has keyboard focus. Display changes and a five-minute editing timeout
cancel safely. A per-user lock prevents simultaneous selector helpers.

The helper re-matches the GTK monitor snapshot against normalized NSScreen frames;
a missing/ambiguous match aborts rather than targeting the wrong display. Tests
cover native coordinate conversion; 7 input-event + 5 GTK-library tests and all
workspace Clippy checks pass. Native UI verification observed the small floating
panel and successful confirm/return (not a fullscreen surface). Actual mouse
passthrough to other applications and every handle/edge combination still need
manual acceptance testing. Non-macOS builds currently retain the fullscreen
fallback; a Windows-native equivalent is not implemented yet.

### Endpoint reachability fix

Native edge panels now override AppKit frame constraining so the menu bar does not
push the top handle down into visibleFrame. Only the thin line and small handles
sit just above status-bar level; the control panel remains floating. Dragging
within 12 logical points of an axis endpoint snaps exactly to 0 or 1, while normal
dragging retains the original grab offset. Native regression tests cover all four
axis endpoints, negative display coordinates, grab offset and frame constraints.
The rebuilt helper is used the next time the overlay opens; an already-running
selection session is deliberately not terminated or discarded.

## Product UI pass (2026-11-09)

Moved the entry into the actual ClientRow template as a per-client range settings
subfeature. Removed the global File-menu editor entry and the large preview canvas.
A compact Libadwaita settings page shows monitor, edge and a single-line range
summary; native on-screen adjustment remains the primary action. Advanced controls
are collapsed. Save stores an endpoint-scoped JSON draft via atomic GFile replacement
with backup and private creation permissions. Hashed storage names prevent host
strings causing path traversal or filename overflow. Duplicate windows for one
endpoint are reused. Monitor layout is revalidated before save; unmatched stored
geometry produces a reselection notice. No daemon schema or protocol is modified.

Validation: 7 input-event tests + 6 GTK-library tests passed, including bounded and
isolated endpoint storage paths. All-feature workspace Clippy and formatting passed.
Native offline preview exercised the real client row -> range settings, advanced
percent editing to 25–100%, save/return, and reopening with that range restored.
The preview uses a fictitious client and does not send IPC to the running daemon.
Production installation and actual portal switching remain intentionally separate.

## Windows native overlay and build pipeline

Windows now launches a separate self-contained .NET 8 WinForms helper. It creates
only a click-through topmost line, two small drag handles and a control panel;
there is no fullscreen surface or global input hook. GDK monitor geometry and its
runtime scale are matched against per-monitor-DPI-aware native Screen bounds;
ambiguous/stale matches fail rather than choosing another screen. Screen changes,
Escape/cancel and a five-minute timeout discard edits. Rust launches it without a
console and validates its JSON result before updating the client draft.

Windows development CI runs on windows-2025 with the MSYS2 UCRT64 Rust/GTK toolchain,
uses the checked-in Cargo.lock, builds/tests the native helper and packages a
self-contained runtime. A separate publish job creates a clearly marked prerelease
only after successful tests and packaged --version checks. See WINDOWS_TEST_BUILD.md
for actual supported scope and manual acceptance tests. No settings are installed
or modified on the user's existing Windows machine by this workflow.
