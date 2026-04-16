# ClipSync TODO & Features Tracker

This document acts as a single source of truth for implemented features and future TODOs across the ClipSync project.

## Implemented Features (Done)

### Phase 1: Foundation UI

- [x] Dashboard and settings UI
- [x] Persistent settings (`max_image_size_kb`, mandatory `pairing_code`)
- [x] Pairing gate enforcement before sync can be enabled

### Phase 2: Discovery

- [x] mDNS service registration + browsing
- [x] UDP broadcast discovery fallback for hotspot scenarios
- [x] Request/reply beacon behavior to improve symmetric peer visibility

### Phase 3: Transport Auth Foundation

- [x] WebSocket transport server/client handshake loop
- [x] Mandatory pairing-code verification in transport handshake
- [x] Per-peer transport status shown in dashboard

### Phase 4: Core Sync

- [x] Authenticated text payload transfer over transport
- [x] Authenticated image payload transfer over transport
- [x] Remote text inbox consumption path
- [x] Remote image inbox consumption path
- [x] Loop-prevention hash cache and drop counting
- [x] Sync counters (sent/received/dropped) in dashboard
- [x] Manual text + image sync test UI + best-effort clipboard polling
- [x] Local-network transport scoping (non-local peers are ignored)

### Phase 5: Hardening

- [x] Deterministic conflict policy with timestamp + sender tie-break
- [x] Stale message rejection tracking (`stale_rejected`)
- [x] Runtime diagnostics event buffer exposed to UI
- [x] Dashboard diagnostics panel for quick field debugging
- [x] Structured SUCCESS/FAILED/INFO logs for text/image send/receive paths
- [x] UI console logging for pairing, sync toggles, settings save, and visibility events
- [x] Native bridge telemetry counters (captured/sent/skipped/failed/malformed) exposed in dashboard

### Phase 6: Background Reliability & Lifecycle

- [x] Optional background reliability mode toggle in Settings
- [x] App foreground/background visibility reporting to backend runtime
- [x] Runtime health panel in dashboard (foreground state + report age)
- [x] Background mode policy affects local sync send behavior
- [x] Stale peer pruning watchdog added (discovery TTL)
- [x] Runtime metrics extended: authenticated peers, last auth age, stale peers pruned

### Phase 7: Android Native Scaffold

- [x] Android foreground service class scaffolded and started from MainActivity
- [x] Boot receiver scaffold added for auto-start on boot
- [x] Accessibility service scaffold + XML metadata added
- [x] Android manifest updated with required service/boot permissions and component declarations

### Phase 8: Android Integration

- [x] Foreground service listens to native clipboard changes and emits bridge events
- [x] Accessibility service publishes best-effort text events for background assist
- [x] MainActivity forwards native clipboard events into WebView as `clipsync-native-clipboard`
- [x] React controller consumes native events and feeds authenticated sync pipeline
- [x] Foreground service lifecycle policy follows background mode setting + app visibility
- [x] Native image clipboard bridge added (URI-based image clipboard payload -> image sync pipeline)

### Phase 9: Verification

- [x] In-app Validation tab with step-11 matrix cases (pass/fail/not-run)
- [x] Per-case notes and last-run timestamps
- [x] JSON export for release readiness evidence

### Phase 10: Release Readiness

- [x] Repeatable RC scripts added (`rc:check`, `rc:desktop`, `rc:android`)
- [x] Internal release checklist added inABOUT_PROJECT/RELEASE_RC_CHECKLIST.md
- [x] Documentation aligned with current known limitations
- [x] Signed Android release install flow documented (PowerShell-safe)

## Future TODOs & Pending Plans

### Phase A: Close partially implemented items

- [x] **Improve image sync reliability and document exact limits**
  - Keep existing manual image sync path stable.
  - Add fallback handling notes for Android apps that do not expose image URI clipboard payloads.
  - Define explicit supported image scenarios (manual picker, URI-based capture, unsupported app cases).
  - Deliverables: Updated README behavior matrix for image sync, Validation test cases.

- [x] **Add missing Android nearby permissions + runtime handling**
  - Add `NEARBY_WIFI_DEVICES` (and companion permissions).
  - Verify no regression for existing permissions.
  - Add first-run permission UX copy for user clarity.
  - Deliverables: Updated Android manifest, Permission request flow verified on Android 13/14/15.

- [x] **Promote boot auto-start from scaffold to verified behavior**
  - Validate `ClipSyncBootReceiver` on cold boot and locked boot cases.
  - Ensure service start behavior follows policy and Android background restrictions safely.
  - Add diagnostics event for boot-triggered start path.
  - Deliverables: Repeatable boot test checklist and pass evidence. Docs updated.

- [x] **Remove ambiguity from "no need to open app daily" claim**
  - Align user-facing copy with real service policy (`background_mode_enabled` + app state).
  - Make expected behavior explicit for ON and OFF background reliability mode.
  - Deliverables: README text updated, In-app hint text aligned.

### Phase B: Implement missing features

- [x] **Add Android notification action to pause/resume sync**
  - Add notification action buttons and PendingIntent in foreground service.
  - Wire action to app/native bridge so sync state toggles without opening UI.
  - Keep action idempotent and reflect in dashboard status.
  - Deliverables: Notification action UX working end-to-end, Validation case.

- [x] **Add Windows auto-start on login**
  - Integrate Tauri auto-start plugin/config in backend and app config.
  - Provide user setting to enable/disable start on login.
  - Validate behavior on fresh install and after reboot.
  - Deliverables: Auto-start capability integrated, Settings toggle.

- [x] **Add Windows system tray icon and menu controls**
  - Create tray icon with menu options: Open, Sync On/Off, Quit.
  - Show connection state via tooltip/title updates.
  - Ensure minimize/close behavior and window restore UX.
  - Deliverables: Tray integration wired, Manual test checklist.

### Execution Order & Acceptance Gates

- [x] Phase A complete with validation evidence.
- [x] Phase B complete with desktop + Android sanity checks.
- [x] README and release notes updated to only claim verified behaviors.
- [x] RC rerun required: `npm run rc:check`, `npm run rc:desktop`, `npm run rc:android`.

## UI/UX Redesign & Dev Mode

### Phase C: Modernize UI and Dev Mode Split (Done)
- [x] Backend: Add `dev_mode: bool` to the Rust application settings (`AppSettings`).
- [x] Frontend State: Connect `devModeEnabled` in API wrapper and controller context.
- [x] Dev Mode Guard: Hide advanced diagnostics, debug metrics, manual sync inputs, and Validation tab unless `devModeEnabled` is `true`.
- [x] App Redesign (Minimal & Modern):
  - Filter `AppShell` and `AppHeader` to show tests only in Dev Mode.
  - Simplify `DashboardView` into a beautiful, user-friendly connection status page.
  - Clean up `SettingsView` and introduce the Developer Mode toggle.
- [x] Polish: Proper CSS integration preserved.
