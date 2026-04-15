import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import './App.css';

type StatusResponse = {
  status: 'searching' | 'connected';
  devices: string[];
  sync_enabled: boolean;
  paired: boolean;
  peer_transport?: Record<string, string>;
  sync_stats?: {
    sent: number;
    received: number;
    dropped: number;
    stale_rejected: number;
  };
};

type SettingsResponse = {
  max_image_size_kb: number;
  pairing_code: string;
  device_name_override: string;
};

type IncomingImage = {
  mime_type: string;
  image_base64: string;
};

function App() {
  const [currentTab, setCurrentTab] = useState<'dashboard' | 'settings'>('dashboard');
  const [syncEnabled, setSyncEnabled] = useState(true);
  const [status, setStatus] = useState<'searching' | 'connected'>('searching');
  const [devices, setDevices] = useState<string[]>([]);
  const [paired, setPaired] = useState(false);
  const [maxImageSizeKb, setMaxImageSizeKb] = useState(2048);
  const [pairingCode, setPairingCode] = useState('');
  const [deviceNameOverride, setDeviceNameOverride] = useState('');
  const [unlockCode, setUnlockCode] = useState('');
  const [saveMessage, setSaveMessage] = useState('');
  const [syncMessage, setSyncMessage] = useState('');
  const [peerTransport, setPeerTransport] = useState<Record<string, string>>({});
  const [syncStats, setSyncStats] = useState({ sent: 0, received: 0, dropped: 0, stale_rejected: 0 });
  const [diagnostics, setDiagnostics] = useState<string[]>([]);
  const [manualSyncText, setManualSyncText] = useState('');
  const [remoteTextPreview, setRemoteTextPreview] = useState('');
  const [manualImagePreview, setManualImagePreview] = useState('');
  const [manualImageMime, setManualImageMime] = useState('image/png');
  const [remoteImagePreview, setRemoteImagePreview] = useState('');
  const lastClipboardTextRef = useRef('');

  // Fetch initial status
  useEffect(() => {
    const fetchStatus = async () => {
      try {
        const res = await invoke<StatusResponse>('get_status');
        setStatus(res.status);
        setDevices(res.devices);
        setSyncEnabled(res.sync_enabled);
        setPaired(res.paired);
        setPeerTransport(res.peer_transport ?? {});
        setSyncStats(res.sync_stats ?? { sent: 0, received: 0, dropped: 0, stale_rejected: 0 });
      } catch (e) {
        console.error(e);
      }
    };

    const fetchDiagnostics = async () => {
      try {
        const events = await invoke<string[]>('get_diagnostics');
        setDiagnostics(events.slice(-8));
      } catch (e) {
        console.error(e);
      }
    };

    const fetchSettings = async () => {
      try {
        const settings = await invoke<SettingsResponse>('get_settings');
        setMaxImageSizeKb(settings.max_image_size_kb);
        setPairingCode(settings.pairing_code);
        setDeviceNameOverride(settings.device_name_override ?? '');
      } catch (e) {
        console.error(e);
      }
    };

    fetchStatus();
    fetchSettings();
    fetchDiagnostics();

    const timer = window.setInterval(() => {
      fetchStatus();
      fetchDiagnostics();
    }, 3000);
    return () => window.clearInterval(timer);
  }, []);

  useEffect(() => {
    const interval = window.setInterval(async () => {
      if (!paired || !syncEnabled) return;

      try {
        const remote = await invoke<string | null>('consume_remote_text');
        if (remote) {
          setRemoteTextPreview(remote);
          try {
            await navigator.clipboard.writeText(remote);
          } catch {
            // Clipboard API can fail on some Android WebView contexts.
          }
        }
      } catch (e) {
        console.error(e);
      }

      try {
        const remoteImage = await invoke<IncomingImage | null>('consume_remote_image');
        if (remoteImage && remoteImage.image_base64) {
          setRemoteImagePreview(`data:${remoteImage.mime_type};base64,${remoteImage.image_base64}`);
        }
      } catch (e) {
        console.error(e);
      }

      try {
        const localText = await navigator.clipboard.readText();
        if (localText && localText !== lastClipboardTextRef.current) {
          lastClipboardTextRef.current = localText;
          await invoke('push_local_text_clipboard', { content: localText });
        }
      } catch {
        // Ignore clipboard read issues and keep manual sync path available.
      }
    }, 1200);

    return () => window.clearInterval(interval);
  }, [paired, syncEnabled]);

  const toggleSync = async () => {
    const newState = !syncEnabled;
    try {
      await invoke('toggle_sync', { enabled: newState });
      setSyncEnabled(newState);
      setSyncMessage('');
    } catch (error) {
      console.error('Failed to toggle sync:', error);
      setSyncMessage(String(error));
    }
  };

  const onSaveSettings = async () => {
    if (!/^\d{4}$/.test(pairingCode)) {
      setSaveMessage('Pairing code must be exactly 4 digits.');
      return;
    }

    try {
      await invoke('save_settings', {
        maxImageSizeKb,
        pairingCode,
        deviceNameOverride,
      });
      setSaveMessage('Settings saved. Device name update may require app restart for discovery name refresh.');
      setPaired(false);
      setSyncEnabled(false);
    } catch (error) {
      console.error('Failed to save settings:', error);
      setSaveMessage('Failed to save settings.');
    }
  };

  const onUnlockSync = async () => {
    if (!/^\d{4}$/.test(unlockCode)) {
      setSyncMessage('Enter your 4-digit pairing code to unlock sync.');
      return;
    }

    try {
      const ok = await invoke<boolean>('validate_pairing', { code: unlockCode });
      if (ok) {
        setPaired(true);
        setSyncMessage('Pairing verified. You can enable sync now.');
      } else {
        setPaired(false);
        setSyncEnabled(false);
        setSyncMessage('Invalid pairing code.');
      }
    } catch (error) {
      console.error('Failed to validate pairing:', error);
      setSyncMessage('Unable to verify pairing code right now.');
    }
  };

  const onManualSync = async () => {
    if (!manualSyncText.trim()) {
      setSyncMessage('Enter text to send.');
      return;
    }
    try {
      await invoke('push_local_text_clipboard', { content: manualSyncText });
      setSyncMessage('Text sent to authenticated peers.');
    } catch (error) {
      console.error('Manual sync failed:', error);
      setSyncMessage('Manual sync failed.');
    }
  };

  const onPickManualImage: React.ChangeEventHandler<HTMLInputElement> = async (event) => {
    const file = event.target.files?.[0];
    if (!file) return;
    setManualImageMime(file.type || 'image/png');

    const reader = new FileReader();
    reader.onload = () => {
      const result = typeof reader.result === 'string' ? reader.result : '';
      setManualImagePreview(result);
    };
    reader.readAsDataURL(file);
  };

  const onManualImageSync = async () => {
    if (!manualImagePreview.startsWith('data:')) {
      setSyncMessage('Pick an image first.');
      return;
    }

    const base64 = manualImagePreview.split(',')[1] ?? '';
    if (!base64) {
      setSyncMessage('Invalid image payload.');
      return;
    }

    try {
      await invoke('push_local_image_payload', {
        imageBase64: base64,
        mimeType: manualImageMime,
      });
      setSyncMessage('Image sent to authenticated peers.');
    } catch (error) {
      console.error('Manual image sync failed:', error);
      setSyncMessage('Manual image sync failed.');
    }
  };

  return (
    <div className="min-h-screen bg-gray-950 text-white flex flex-col">
      <div className="bg-gray-900 border-b border-gray-800 px-6 py-4 flex items-center justify-between">
        <div className="flex items-center gap-3">
          <span className="text-3xl">📋</span>
          <h1 className="text-2xl font-bold">ClipSync</h1>
        </div>
        
        <div className="flex gap-1 bg-gray-800 rounded-xl p-1">
          <button onClick={() => setCurrentTab('dashboard')} className={`px-6 py-2 rounded-xl font-medium ${currentTab === 'dashboard' ? 'bg-gray-900 text-white' : 'text-gray-400'}`}>Dashboard</button>
          <button onClick={() => setCurrentTab('settings')} className={`px-6 py-2 rounded-xl font-medium ${currentTab === 'settings' ? 'bg-gray-900 text-white' : 'text-gray-400'}`}>Settings</button>
        </div>
      </div>

      <div className="flex-1 p-8">
        {currentTab === 'dashboard' && (
          <div className="max-w-2xl mx-auto space-y-6">
            <div className="bg-gray-900 rounded-3xl p-10 text-center">
              <div className={`text-7xl mb-6 ${status === 'connected' ? 'text-green-400' : 'text-yellow-400'}`}>
                {status === 'connected' ? '✅' : '🔎'}
              </div>
              <h2 className="text-5xl font-semibold mb-3">
                {status === 'connected' ? 'Connected & Syncing' : 'Searching for devices...'}
              </h2>
              <p className="text-gray-400 text-xl mb-8">
                {status === 'connected' 
                  ? 'Your devices are connected over local network.' 
                  : 'Open ClipSync on your Android phone on the same Wi-Fi or hotspot.'}
              </p>

              <p className="text-gray-400 mb-6">
                Security status: {paired ? 'Paired' : 'Locked (pairing required)'}
              </p>

              <div className="unlock-row">
                <input
                  type="text"
                  maxLength={4}
                  value={unlockCode}
                  onChange={(e) => setUnlockCode(e.target.value.replace(/\D/g, ''))}
                  className="settings-input"
                  placeholder="Enter 4-digit code"
                />
                <button onClick={onUnlockSync} className="unlock-btn">Unlock Sync</button>
              </div>

              <button
                onClick={toggleSync}
                className={`px-12 py-4 rounded-2xl text-xl font-medium ${syncEnabled ? 'bg-green-600 hover:bg-green-500' : 'bg-gray-700 hover:bg-gray-600'}`}
                disabled={!paired && !syncEnabled}
              >
                {syncEnabled ? '✅ Sync Enabled' : '❌ Sync Disabled'}
              </button>
              {syncMessage ? <p className="settings-hint mt-4">{syncMessage}</p> : null}
            </div>

            <div className="bg-gray-900 rounded-3xl p-8">
              <h3 className="text-xl font-medium mb-4">Discovered Devices</h3>
              {devices.length > 0 ? (
                devices.map((device, i) => (
                  <div key={i} className="bg-gray-800 p-4 rounded-2xl mb-2 flex items-center gap-3">
                    <div className="w-3 h-3 bg-green-400 rounded-full animate-pulse"></div>
                    <div>
                      <div>{device}</div>
                      <div className="text-xs text-gray-400 mt-4">
                        Transport: {peerTransport[device] ?? 'discovered (handshake pending)'}
                      </div>
                    </div>
                  </div>
                ))
              ) : (
                <div className="bg-gray-800 rounded-2xl p-8 text-center text-gray-400">
                  No devices found yet.<br />Waiting for your phone...
                </div>
              )}

              <div className="sync-stats-box">
                <div className="text-gray-400">Sync Stats</div>
                <div className="sync-stats-grid">
                  <div>Sent: {syncStats.sent}</div>
                  <div>Received: {syncStats.received}</div>
                  <div>Dropped: {syncStats.dropped}</div>
                  <div>Stale Rejected: {syncStats.stale_rejected}</div>
                </div>
              </div>

              <div className="sync-stats-box">
                <div className="text-gray-400">Recent Diagnostics</div>
                {diagnostics.length > 0 ? (
                  diagnostics.map((event, idx) => (
                    <div key={idx} className="diagnostic-row">{event}</div>
                  ))
                ) : (
                  <div className="diagnostic-row">No diagnostics yet</div>
                )}
              </div>

              <div className="manual-sync-box">
                <label className="settings-label" htmlFor="manualSyncText">Manual text sync test</label>
                <textarea
                  id="manualSyncText"
                  value={manualSyncText}
                  onChange={(e) => setManualSyncText(e.target.value)}
                  className="settings-input manual-sync-input"
                  placeholder="Type text and send to peer"
                />
                <button onClick={onManualSync} className="settings-save-btn">Send Text</button>
                <p className="settings-hint">Last remote text: {remoteTextPreview || 'No remote text yet'}</p>
              </div>

              <div className="manual-sync-box">
                <label className="settings-label" htmlFor="manualImage">Manual image sync test</label>
                <input
                  id="manualImage"
                  type="file"
                  accept="image/*"
                  onChange={onPickManualImage}
                  className="settings-input"
                />
                {manualImagePreview ? (
                  <img src={manualImagePreview} alt="Manual to send" className="sync-image-preview" />
                ) : null}
                <button onClick={onManualImageSync} className="settings-save-btn">Send Image</button>
                {remoteImagePreview ? (
                  <>
                    <p className="settings-hint">Last remote image:</p>
                    <img src={remoteImagePreview} alt="Remote" className="sync-image-preview" />
                  </>
                ) : (
                  <p className="settings-hint">No remote image yet</p>
                )}
              </div>
            </div>
          </div>
        )}

        {currentTab === 'settings' && (
          <div className="max-w-2xl mx-auto bg-gray-900 rounded-3xl p-10">
            <h2 className="text-3xl font-semibold mb-6">Settings</h2>
            <div className="space-y-4">
              <label className="settings-label" htmlFor="maxImageSizeKb">
                Max image size to sync (KB)
              </label>
              <input
                id="maxImageSizeKb"
                type="number"
                min={64}
                step={64}
                value={maxImageSizeKb}
                onChange={(e) => setMaxImageSizeKb(Number(e.target.value))}
                className="settings-input"
              />

              <label className="settings-label" htmlFor="pairingCode">
                Mandatory pairing code (4 digits)
              </label>
              <input
                id="pairingCode"
                type="text"
                maxLength={4}
                value={pairingCode}
                onChange={(e) => setPairingCode(e.target.value.replace(/\D/g, ''))}
                className="settings-input"
                placeholder="0000"
              />

              <label className="settings-label" htmlFor="deviceNameOverride">
                Device name (optional)
              </label>
              <input
                id="deviceNameOverride"
                type="text"
                maxLength={40}
                value={deviceNameOverride}
                onChange={(e) => setDeviceNameOverride(e.target.value)}
                className="settings-input"
                placeholder="Leave empty to use default name"
              />

              <button onClick={onSaveSettings} className="settings-save-btn">
                Save Settings
              </button>
              {saveMessage ? <p className="settings-hint">{saveMessage}</p> : null}
            </div>
          </div>
        )}
      </div>

      <div className="text-center text-xs text-gray-500 py-6 border-t border-gray-800">
        Local network only • Privacy-first
      </div>
    </div>
  );
}

export default App;