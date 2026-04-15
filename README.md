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

## What works right now

- Both devices can discover each other over local network (Wi-Fi/hotspot).
- Both devices can attempt authenticated transport handshake.
- UI shows transport result per discovered peer:
  - `authenticated with ...`
  - `rejected: pairing mismatch`
  - connection/ack errors when applicable
- Text payload sync is active for authenticated peers.
- Image payload sync is active for authenticated peers (manual image path).
- Sync counters are visible in dashboard.
- Conflict and stale message decisions are visible via diagnostics.

## What is not implemented yet

- Android background clipboard Accessibility bridge is not wired yet.

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

## Notes

- Discovery currently updates backend state; UI polls every 3 seconds to reflect newly discovered peers.
- Transport handshake is now active and should show status like `authenticated` or `rejected: pairing mismatch` per peer.
- Text and image sync use authenticated transport and loop-prevention hashing.
- Conflict resolution now prefers newer timestamp; tie uses sender id ordering for deterministic behavior.
- Android background clipboard bridge is the next implementation step.
