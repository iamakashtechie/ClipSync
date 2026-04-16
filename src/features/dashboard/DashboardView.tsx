import type { ChangeEventHandler } from 'react';
import type { NativeBridgeStats, RuntimeHealth, SyncStats, SyncStatus } from '../../shared/types/clipsync';

type DashboardViewProps = {
  devModeEnabled?: boolean;
  status: SyncStatus;
  paired: boolean;
  unlockCode: string;
  onUnlockCodeChange: (value: string) => void;
  onUnlockSync: () => void;
  syncEnabled: boolean;
  onToggleSync: () => void;
  syncMessage: string;
  devices: string[];
  peerTransport: Record<string, string>;
  syncStats: SyncStats;
  diagnostics: string[];
  runtimeHealth: RuntimeHealth;
  manualSyncText: string;
  onManualSyncTextChange: (value: string) => void;
  onManualSync: () => void;
  remoteTextPreview: string;
  nativeBridgeStatus: string;
  nativeBridgeStats: NativeBridgeStats;
  onPickManualImage: ChangeEventHandler<HTMLInputElement>;
  manualImagePreview: string;
  onManualImageSync: () => void;
  remoteImagePreview: string;
};

export function DashboardView({
  devModeEnabled = false,
  status,
  paired,
  unlockCode,
  onUnlockCodeChange,
  onUnlockSync,
  syncEnabled,
  onToggleSync,
  syncMessage,
  devices,
  peerTransport,
  syncStats,
  diagnostics,
  runtimeHealth,
  manualSyncText,
  onManualSyncTextChange,
  onManualSync,
  remoteTextPreview,
  nativeBridgeStatus,
  nativeBridgeStats,
  onPickManualImage,
  manualImagePreview,
  onManualImageSync,
  remoteImagePreview,
}: DashboardViewProps) {
  return (
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
            onChange={(event) => onUnlockCodeChange(event.target.value.replace(/\D/g, ''))}
            className="settings-input"
            placeholder="Enter 4-digit code"
          />
          <button onClick={onUnlockSync} className="unlock-btn">Unlock Sync</button>
        </div>

        <button
          onClick={onToggleSync}
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
          devices.map((device, index) => (
            <div key={`${device}-${index}`} className="bg-gray-800 p-4 rounded-2xl mb-2 flex items-center gap-3">
              <div className="w-3 h-3 bg-green-400 rounded-full animate-pulse"></div>
              <div>
                <div>{device}</div>
                {devModeEnabled && (
                  <div className="text-xs text-gray-400 mt-4">
                    Transport: {peerTransport[device] ?? 'discovered (handshake pending)'}
                  </div>
                )}
              </div>
            </div>
          ))
        ) : (
          <div className="bg-gray-800 rounded-2xl p-8 text-center text-gray-400">
            No devices found yet.<br />Waiting for your phone...
          </div>
        )}

        {devModeEnabled && (
          <div style={{ marginTop: '2rem', borderTop: '1px solid rgba(255,255,255,0.1)', paddingTop: '1.5rem' }}>
            <h3 style={{ fontSize: '1.1rem', fontWeight: '500', marginBottom: '1rem', color: '#8698af' }}>Developer Tools</h3>
            
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
                diagnostics.map((event, index) => (
                  <div key={`${event}-${index}`} className="diagnostic-row">{event}</div>
                ))
              ) : (
                <div className="diagnostic-row">No diagnostics yet</div>
              )}
            </div>

            <div className="sync-stats-box">
              <div className="text-gray-400">Runtime Health</div>
              <div className="sync-stats-grid">
                <div>App: {runtimeHealth.is_app_foreground ? 'Foreground' : 'Background'}</div>
                <div>Report age: {Math.round(runtimeHealth.visibility_report_age_ms / 1000)}s</div>
                <div>Bg mode: {runtimeHealth.background_mode_enabled ? 'Enabled' : 'Disabled'}</div>
                <div>
                  Last auth: {runtimeHealth.last_auth_age_ms === Number.MAX_SAFE_INTEGER
                    ? 'n/a'
                    : `${Math.round(runtimeHealth.last_auth_age_ms / 1000)}s ago`}
                </div>
                <div>Auth peers: {runtimeHealth.authenticated_peer_count}</div>
                <div>Pruned peers: {runtimeHealth.stale_peers_pruned}</div>
              </div>
              <p className="settings-hint mt-4">
                Policy: {runtimeHealth.background_mode_enabled
                  ? 'Background mode is ON, so Android keeps foreground service behavior while app is backgrounded.'
                  : 'Background mode is OFF, so background service is stopped and app should be reopened for active sync.'}
              </p>
            </div>

            <div className="sync-stats-box">
              <div className="text-gray-400">Native Bridge Status (Android)</div>
              <div className="diagnostic-row">{nativeBridgeStatus}</div>
              <div className="sync-stats-grid mt-4">
                <div>Captured text: {nativeBridgeStats.captured_text}</div>
                <div>Captured image: {nativeBridgeStats.captured_image}</div>
                <div>Sent text: {nativeBridgeStats.sent_text}</div>
                <div>Sent image: {nativeBridgeStats.sent_image}</div>
                <div>Skipped: {nativeBridgeStats.skipped}</div>
                <div>Failed: {nativeBridgeStats.failed}</div>
                <div>Malformed: {nativeBridgeStats.malformed}</div>
                <div>Last source: {nativeBridgeStats.last_source}</div>
                <div>Last type: {nativeBridgeStats.last_type}</div>
              </div>
            </div>

            <div className="manual-sync-box">
              <label className="settings-label" htmlFor="manualSyncText">Manual text sync test</label>
              <textarea
                id="manualSyncText"
                value={manualSyncText}
                onChange={(event) => onManualSyncTextChange(event.target.value)}
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
        )}
      </div>
    </div>
  );
}
