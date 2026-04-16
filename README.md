# ClipSync

Local clipboard sync between Windows and Android using Tauri + Rust + React.

## Current implementation status

- Phase 1 complete:
  - Dashboard and settings UI
  - Persistent settings (`max_image_size_kb`, mandatory `pairing_code`)
  - Pairing gate enforcement before sync can be enabled
- Phase 2 complete (discovery):
  - mDNS service registration + browsing
  - UDP broadcast discovery fallback for hotspot scenarios
  - Request/reply beacon behavior to improve symmetric peer visibility
- Phase 3 foundation complete (transport auth):
  - WebSocket transport server/client handshake loop
  - Mandatory pairing-code verification in transport handshake
  - Per-peer transport status shown in dashboard
- Phase 4 (current sync-core milestone):
  - Authenticated text payload transfer over transport
  - Authenticated image payload transfer over transport
  - Remote text inbox consumption path
  - Remote image inbox consumption path
  - Loop-prevention hash cache and drop counting
  - Sync counters (sent/received/dropped) in dashboard
  - Manual text + image sync test UI + best-effort clipboard polling
  - Local-network transport scoping (non-local peers are ignored)
- Phase 5 hardening started:
  - Deterministic conflict policy with timestamp + sender tie-break
  - Stale message rejection tracking (`stale_rejected`)
  - Runtime diagnostics event buffer exposed to UI
  - Dashboard diagnostics panel for quick field debugging
  - Structured SUCCESS/FAILED/INFO logs for text/image send/receive paths
  - UI console logging for pairing, sync toggles, settings save, and visibility events
  - Native bridge telemetry counters (captured/sent/skipped/failed/malformed) exposed in dashboard
- Phase 6 groundwork started (background reliability):
  - Optional background reliability mode toggle in Settings
  - App foreground/background visibility reporting to backend runtime
  - Runtime health panel in dashboard (foreground state + report age)
- Phase 6 reliability integration expanded:
  - Background mode policy now affects local sync send behavior
  - Stale peer pruning watchdog added (discovery TTL)
  - Runtime metrics extended: authenticated peers, last auth age, stale peers pruned
- Phase 7 Android native scaffold started:
  - Android foreground service class scaffolded and started from MainActivity
  - Boot receiver scaffold added for auto-start on boot
  - Accessibility service scaffold + XML metadata added
  - Android manifest updated with required service/boot permissions and component declarations
- Phase 8 integration progressed:
  - Foreground service now listens to native clipboard changes and emits bridge events
  - Accessibility service now publishes best-effort text events for background assist
  - MainActivity now forwards native clipboard events into WebView as `clipsync-native-clipboard`
  - React controller now consumes native events and feeds authenticated sync pipeline
  - Foreground service lifecycle policy now follows background mode setting + app visibility
  - Native image clipboard bridge added (URI-based image clipboard payload -> image sync pipeline)
- Phase 9 verification tooling started:
  - In-app Validation tab with step-11 matrix cases (pass/fail/not-run)
  - Per-case notes and last-run timestamps
  - JSON export for release readiness evidence
- Phase 10 release readiness started:
  - Repeatable RC scripts added (`rc:check`, `rc:desktop`, `rc:android`)
  - Internal release checklist added in ABOUT_PROJECT/RELEASE_RC_CHECKLIST.md
  - Documentation aligned with current known limitations
  - Signed Android release install flow documented (PowerShell-safe)

## What works right now

- Both devices can discover each other over local network (Wi-Fi/hotspot).
- Both devices can attempt authenticated transport handshake.
- UI shows transport result per discovered peer:
  - `authenticated with ...`
  - `rejected: pairing mismatch`
  - connection/ack errors when applicable
- Text payload sync is active for authenticated peers.
- Image payload sync is active for authenticated peers (manual image path).
- Image sync now enforces the configured `max_image_size_kb` limit across manual and native send paths.
- Sync counters are visible in dashboard.
- Conflict and stale message decisions are visible via diagnostics.
- Runtime health state is visible for lifecycle debugging.
- Console now reports function-level success/failure events even if UI state glitches.
- Android native service scaffolding is present in generated Android module.
- Background mode now actively gates local send when app is in background.
- Android manifest now includes nearby-network permission declarations with API-level compatibility handling.
- Boot receiver now follows background-mode policy and emits boot runtime diagnostics to the app UI.
- Android foreground notification now includes `Pause Sync` / `Resume Sync` action with runtime status propagation.
- Windows settings now include `Start ClipSync on Windows login` with persisted autostart behavior.
- Windows tray icon now provides `Open`, `Sync On/Off`, and `Quit` controls with connection-aware tooltip updates.

## What is not implemented yet

- Accessibility bridge currently forwards best-effort text events only (image/background write path still pending).
- Some Android apps do not place copy-image content on system clipboard as shareable URI; in those cases native image capture may not trigger.

## Notification action behavior (Phase B1 in progress)

- Foreground notification provides one action button that flips between `Pause Sync` and `Resume Sync`.
- Action is idempotent: pressing pause while already paused (or resume while active) keeps state stable and logs an info event.
- Action writes native sync state immediately and emits runtime event so dashboard/native status can reflect the toggle.

## Windows autostart behavior (Phase B2 in progress)

- Settings now include `Start ClipSync on Windows login`.
- Save applies autostart immediately on Windows and persists choice in app settings.
- App startup also reconciles autostart state to match saved setting.

## Windows tray behavior (Phase B3 in progress)

- Tray menu includes: `Open`, `Sync On/Off`, and `Quit`.
- Tray tooltip is refreshed with runtime connection state (`Sync`, discovered peer count, authenticated peer count).
- Closing main window hides app to tray on Windows; `Open` restores and focuses the main window.

## Boot auto-start behavior (Phase A3 in progress)

- On `BOOT_COMPLETED`, boot receiver starts the foreground service only when `background_mode_enabled=true`.
- On `LOCKED_BOOT_COMPLETED`, boot receiver defers service start and emits a runtime diagnostics event.
- Boot path now publishes runtime diagnostics (`SUCCESS` / `INFO` / `FAILED`) into the app's Native Bridge status stream.

## Background reliability policy (explicit)

- `background_mode_enabled = ON`:
  - While app is backgrounded, Android foreground service remains active.
  - This is the mode intended for "no need to reopen app each time" behavior, subject to Android battery/device restrictions.
- `background_mode_enabled = OFF`:
  - Foreground service is stopped when app is backgrounded.
  - Reopen ClipSync app to resume active sync and runtime updates.

## Android permission behavior (Phase A2 in progress)

- Android 13+ requests `NEARBY_WIFI_DEVICES` for nearby network discovery compatibility.
- Android 12 and below requests `ACCESS_FINE_LOCATION` as compatibility fallback where older nearby behaviors require it.
- Android 13+ requests `POST_NOTIFICATIONS` when foreground service needs notification visibility.
- App now includes first-run UX guidance in Settings for permission prompts.

## Image sync behavior matrix (Phase A1)

| Scenario category | Source behavior | Expected ClipSync result |
| --- | --- | --- |
| Supported: Manual picker (all devices) | User picks image from `Manual image sync test` | Image is sent if pairing + sync are enabled and payload is within `max_image_size_kb`. |
| Supported: Android URI clipboard image | Android app exposes copied image as shareable clipboard URI with `image/*` MIME | Foreground service reads URI, bridge forwards payload, and image is sent if within `max_image_size_kb`. |
| Unsupported: No URI image in clipboard | Android app copies image in a private/inaccessible format or non-URI payload | Native image capture does not trigger; use manual picker fallback path. |
| Unsupported: Oversized payload | Selected or captured image exceeds configured limit | Payload is rejected locally with diagnostics/status update; app remains stable and continues syncing other payloads. |

## Run

### Frontend build check

```powershell
npm run build
```

### Tauri desktop dev

```powershell
cd src-tauri
cargo check -q
cd ..
npm run tauri dev
```

### Android setup (once)

```powershell
npm run tauri android init
```

### Android dev run

```powershell
npm run tauri android dev
```

### Release candidate checks

```powershell
npm run rc:check
```

### Release candidate desktop package

```powershell
npm run rc:desktop
```

### Release candidate Android package

```powershell
npm run rc:android
```

### Android release APK signing and install (PowerShell)

Release output from `rc:android` is commonly unsigned. Installing
`app-universal-release-unsigned.apk` directly will fail with invalid package errors.

### Permanent automatic signing setup (recommended)

Create `src-tauri/gen/android/keystore.properties` (local-only, ignored by git):

```properties
storeFile=../../release-keystore.jks
storePassword=YOUR_STORE_PASSWORD
keyAlias=clipsync
keyPassword=YOUR_KEY_PASSWORD
```

After this, `npm run rc:android` will use the release signing config automatically.

### Manual sign flow (fallback)

Use this one-time keystore setup:

```powershell
keytool -genkeypair -v -keystore ".\release-keystore.jks" -alias clipsync -keyalg RSA -keysize 2048 -validity 10000
```

If the generated APK is unsigned, align, sign, verify, and install with:

```powershell
$sdkRoot = if ($env:ANDROID_HOME) { $env:ANDROID_HOME } else { $env:ANDROID_SDK_ROOT }
$buildTools = Join-Path $sdkRoot "build-tools\36.0.0"

& (Join-Path $buildTools "zipalign.exe") -v -p 4 "src-tauri\gen\android\app\build\outputs\apk\universal\release\app-universal-release-unsigned.apk" "src-tauri\gen\android\app\build\outputs\apk\universal\release\app-universal-release-aligned.apk"

& (Join-Path $buildTools "apksigner.bat") sign --ks ".\release-keystore.jks" --ks-key-alias clipsync "src-tauri\gen\android\app\build\outputs\apk\universal\release\app-universal-release-aligned.apk"

& (Join-Path $buildTools "apksigner.bat") verify --verbose --print-certs "src-tauri\gen\android\app\build\outputs\apk\universal\release\app-universal-release-aligned.apk"

adb install -r "src-tauri\gen\android\app\build\outputs\apk\universal\release\app-universal-release-aligned.apk"
```

If you only need quick device testing, install debug APK instead:

```powershell
adb install -r "src-tauri\gen\android\app\build\outputs\apk\arm64\debug\app-arm64-debug.apk"
```

## Manual verification (current milestone)

1. Use the same pairing code on both devices and save.
2. Unlock sync on both dashboards with that code.
3. Wait 20-30 seconds with both apps open.
4. Confirm both peers are visible in `Discovered Devices`.
5. Confirm transport status becomes `authenticated with ...` on both devices.
6. Change one device to a different code and save.
7. Confirm transport status changes to `rejected: pairing mismatch`.
8. Enter text in `Manual text sync test` on device A and press `Send Text`.
9. Confirm device B shows the new text in `Last remote text` and `Received` counter increments.
10. Confirm device A `Sent` counter increments.
11. Pick an image in `Manual image sync test` on device A and press `Send Image`.
12. Confirm device B shows `Last remote image` preview and `Received` increments again.
13. Trigger rapid repeated sends from both sides and confirm `Stale Rejected` and diagnostics rows update.
14. Switch app in/out of focus and confirm `Runtime Health` foreground/background and report age updates.
15. Toggle `Background reliability mode (preview)` in Settings, save, and confirm runtime reflects it.
16. Launch Android app and confirm persistent foreground notification appears for ClipSync.
17. Reboot device (optional) and verify service scaffold auto-start behavior.
18. Open Android Accessibility settings and confirm ClipSync service entry exists.
19. Disable `Background reliability mode`, switch app to background, then send local text/image; confirm diagnostics show blocked local sync in background.
20. Keep one peer offline for >30s and confirm `Pruned peers` increments.
21. Open `Validation` tab, mark each scenario pass/fail with notes, and export JSON report.
22. Copy image from an Android app known to expose clipboard URI and confirm native image capture path sends image.
23. Copy image from an Android app that does not expose URI clipboard image and confirm no crash; verify manual picker fallback still works.
24. Try an image above configured `max_image_size_kb`; confirm send is rejected with diagnostics/native bridge status.
25. On Android 13/14/15 fresh install, verify first-run nearby permission prompt and notification permission prompt behavior.
26. On Android 12 test device/emulator, verify compatibility fallback prompt for location permission.
27. Enable `Background reliability mode`, reboot Android device, open app, and confirm Native Bridge Status includes boot success event.
28. Disable `Background reliability mode`, reboot device, open app, and confirm Native Bridge Status reports boot skip due to policy.
29. If locked-boot broadcast is available on device, confirm Native Bridge Status reports locked-boot defer event.
30. With background mode ON, press `Pause Sync` in Android notification and confirm notification text switches to paused state.
31. While paused, copy text/image and confirm no new native clipboard send events are emitted.
32. Press `Resume Sync` in notification and confirm clipboard sends resume and dashboard sync state updates.
33. On Windows desktop, enable `Start ClipSync on Windows login`, save settings, restart Windows session, and confirm ClipSync starts automatically.
34. Disable `Start ClipSync on Windows login`, save settings, restart Windows session, and confirm ClipSync does not auto-start.
35. On Windows, close app window and confirm app stays available in tray instead of fully exiting.
36. From tray menu, click `Open` and confirm main window is restored and focused.
37. From tray menu, click `Sync On/Off` and confirm dashboard sync state and tray tooltip reflect the new state.
38. From tray menu, click `Quit` and confirm process exits completely.

## Notes

- Discovery currently updates backend state; UI polls every 3 seconds to reflect newly discovered peers.
- Transport handshake is now active and should show status like `authenticated` or `rejected: pairing mismatch` per peer.
- Text and image sync use authenticated transport and loop-prevention hashing.
- Conflict resolution now prefers newer timestamp; tie uses sender id ordering for deterministic behavior.
- Reliability watchdog prunes stale peers and reports connection health telemetry.
- Backend and UI logs now use consistent tags (for example: `TEXT_SENT_MANUAL`, `IMAGE_RECEIVED`, `PAYLOAD_SEND`).
- Native Android background components are now integrated with policy-controlled lifecycle behavior.
- Foreground service is policy-controlled: when background mode is enabled and app is backgrounded, service stays active; otherwise it is stopped.
- Daily-use expectation is policy-based: ON favors continuous background behavior; OFF expects reopening app for active sync sessions.
- Android native clipboard bridge now forwards text and URI-based image payloads to frontend/runtime.
- Image sends are rejected when estimated payload size exceeds configured `max_image_size_kb` to prevent partial/corrupted sync attempts.
- Dashboard now includes native bridge observability counters for failure triage and delivery debugging.
- Boot-triggered start path now emits explicit runtime diagnostics that are shown in Native Bridge Status for field validation.
- Validation tab stores matrix progress locally and can export a report for release checks.

## Known limitations (current RC)

- Accessibility path is best-effort text oriented; broad image/background write parity is incomplete.
- Some Android apps do not expose copied images as clipboard URI, so native image capture may not trigger.
- Native clipboard image capture depends on URI-accessible image data; apps using private clipboard containers require manual picker fallback.

## Implementation & TODOs

Please see [TODO.md](TODO.md) for the complete list of implemented features, gap closure plans, and future work.
