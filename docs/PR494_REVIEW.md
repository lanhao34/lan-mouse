# Review of upstream PR #494

Reviewed 2026-11-09, exact head `21297b089badcbb0e49af6a4819e534a9ca9c2b5` (still open).
PR: https://github.com/feschber/lan-mouse/pull/494

Recommendation: retain as a reference branch; integrate with fixes rather than ship as-is for the Mac/Windows portal feature.

## Findings

### P2: Windows warp failures are silently treated as success

`input-emulation/src/windows.rs:110` discards `SetCursorPos`'s Result and returns Ok. For example, a target desktop that cannot accept the warp leaves the pointer at its previous position, defeating positioned transitions with no error surfaced. In addition, `src/emulation.rs:193-202` enqueues a warp and acknowledges the transition before knowing its result. Queue ordering preserves warp-before-input processing, but does not prove a successful warp. Propagate the OS error and define ACK semantics explicitly: receipt versus applied positioning; on failure report/fallback or negatively acknowledge, rather than claim the requested placement occurred.

### P2: duplicate and stale transitions receive indistinguishable successful ACKs

`src/emulation.rs:184-187` uses `!accepts()` for both exact duplicate and genuinely stale transitions, replying with the rejected packet's serial in both cases. Exact duplicate ACKs are needed after lost ACKs. A stale transition should not receive an apparent success for positioning that was not applied. The sender (`src/capture.rs:288`, `464-465`) accepts a matching serial as success without epoch in the ACK. Separate same-transition retries from stale messages; carry complete transition identity in modern acknowledgements or explicitly define a stale response. A delayed stale duplicate in an otherwise healthy stream will generally be ignored by a newer waiting sender, so this is not evidence that every delayed packet moves the pointer incorrectly. The failure matters when peers' pending transition/session state differs or identifiers are reused.

### P3: serial wrap is incompatible with receiver ordering

The sender increments with `wrapping_add(1).max(1)` without changing epoch (`src/capture.rs:364-365`). Receiver ordering requires strictly greater serial for the same epoch (`src/emulation.rs:154`). After u32::MAX, new transitions are rejected while ACKed as above. This needs billions of transitions and is not the likely cause of today's jumps. Increment/renew epoch on wrap, or use defined wrapping comparison with a bounded receive window. An isolated test extracted the actual IncomingTransition implementation and confirmed rejection of serial 1 after u32::MAX.

## Missing functionality for this project (not undisclosed upstream bugs)

- Mac capture explicitly supplies `cross_axis: None` (`input-capture/src/macos.rs:525`).
- Mac emulation does not override `warp_cursor`; the trait default returns Ok without moving (`input-emulation/src/lib.rs:258-264`).
- A receiver advertises positioned transitions regardless of backend support. Distinguish understanding the message format from actually supporting placement, or expose a documented degraded mode.
- Geometry concatenates exposed desktop-edge segments. It does not identify a user-selected monitor/interval, physical dimensions, local alignment anchors, or a paired portal.
- Return capture must be restricted to the paired destination interval, not the entire right/left edge. Coordinate warping alone does not implement this.
- Overlay editing, endpoint dragging, monitor stable IDs, DPI/rotation conversion, geometry revisioning, and apply/cancel behavior are not supplied.

## Integration and validation plan

1. Keep `feature/personal-kvm` as the current reviewed development baseline, including PR #538 and local geometry. Keep PR #494 unchanged on `review/edge-position`.
2. Integrate protocol/geometry on a new integration branch. A non-mutating `git merge-tree` trial reports a conflict in `src/capture.rs`: manually retain positioned Enter retries AND outgoing remap/scroll transformations, old-mapping release on reload, and existing enter/leave hooks. Do not resolve by taking either file wholesale.
3. Fix the findings; test duplicate ACK loss, stale/old-session packets, wrap, unsupported backend, failed warps, old/new peers, and key release across remap reloads.
4. Implement Mac capture fraction and destination placement, with matching Windows behavior. Validate two-way return with native and injected input, multiple displays, negative origins, differing DPI and sleep/display changes.
5. Add paired local portals and wire the existing EdgeMapping API into configuration/capture/emulation. Each peer owns its interval and calibration; shared identity/fraction/orientation makes the pairing unambiguous.
6. Add physical-size/alignment editor and edit-only local overlay after protocol behavior is stable.

Validation evidence: the PR geometry's 25 tests passed in the previous stage. This review additionally extracted and exercised the exact transition comparison implementation (2 characterization tests). No full PR integration build or live Mac/Windows deployment was performed. Existing baseline tests/checks are recorded in PERSONAL_KVM.md. Findings are source review plus these bounded tests, not reproduced desktop interactions.

## Fork maintenance

`upstream`: https://github.com/feschber/lan-mouse.git
`origin`: https://github.com/lanhao34/lan-mouse.git

Keep upstream main unchanged. Push personal development on `feature/personal-kvm`; preserve third-party commit authorship. Record exact PR SHAs before updates, review changes before integration, and release test binaries separately from the installed applications. No automatic updates or deployments are configured.
