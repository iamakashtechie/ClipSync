# ClipSync

Local clipboard sync between Windows and Android using Tauri + Rust + React.

## Current implementation status

- Phase 1 complete:
	- Dashboard and settings UI
	- Persistent settings (`max_image_size_kb`, mandatory `pairing_code`)
	- Pairing gate enforcement before sync can be enabled
- Phase 2 foundation complete:
	- mDNS service registration + browsing
	- UDP broadcast discovery fallback for hotspot scenarios

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

## Notes

- Discovery currently updates backend state; UI polls every 3 seconds to reflect newly discovered peers.
- Clipboard transport/sync protocol and Android background clipboard bridge are the next implementation steps.
