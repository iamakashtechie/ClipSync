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

## What works right now

- Both devices can discover each other over local network (Wi-Fi/hotspot).
- Both devices can attempt authenticated transport handshake.
- UI shows transport result per discovered peer:
	- `authenticated with ...`
	- `rejected: pairing mismatch`
	- connection/ack errors when applicable

## What is not implemented yet

- Clipboard payload sync (text/image transfer) is not wired yet.
- Loop-prevention hash cache is not wired yet.
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

## Notes

- Discovery currently updates backend state; UI polls every 3 seconds to reflect newly discovered peers.
- Transport handshake is now active and should show status like `authenticated` or `rejected: pairing mismatch` per peer.
- Clipboard payload sync and Android background clipboard bridge are the next implementation steps.
