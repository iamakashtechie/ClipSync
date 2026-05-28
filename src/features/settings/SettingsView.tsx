type SettingsViewProps = {
  maxImageSizeKb: number;
  onMaxImageSizeKbChange: (value: number) => void;
  
  
  deviceNameOverride: string;
  onDeviceNameOverrideChange: (value: string) => void;
  backgroundModeEnabled: boolean;
  onBackgroundModeEnabledChange: (value: boolean) => void;
  windowsStartOnLogin: boolean;
  onWindowsStartOnLoginChange: (value: boolean) => void;
  devModeEnabled: boolean;
  onDevModeEnabledChange: (value: boolean) => void;
  onSaveSettings: () => void;
  saveMessage: string;
};

export function SettingsView({
  maxImageSizeKb,
  onMaxImageSizeKbChange,
  
  
  deviceNameOverride,
  onDeviceNameOverrideChange,
  backgroundModeEnabled,
  onBackgroundModeEnabledChange,
  windowsStartOnLogin,
  onWindowsStartOnLoginChange,
  devModeEnabled,
  onDevModeEnabledChange,
  onSaveSettings,
  saveMessage,
}: SettingsViewProps) {
  return (
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
          onChange={(event) => onMaxImageSizeKbChange(Number(event.target.value))}
          className="settings-input"
        />

        

        <label className="settings-label" htmlFor="deviceNameOverride">
          Device name (optional)
        </label>
        <input
          id="deviceNameOverride"
          type="text"
          maxLength={40}
          value={deviceNameOverride}
          onChange={(event) => onDeviceNameOverrideChange(event.target.value)}
          className="settings-input"
          placeholder="Leave empty to use default name"
        />

        <label className="settings-checkbox-row" htmlFor="backgroundModeEnabled">
          <input
            id="backgroundModeEnabled"
            type="checkbox"
            checked={backgroundModeEnabled}
            onChange={(event) => onBackgroundModeEnabledChange(event.target.checked)}
          />
          <span>Background reliability mode</span>
        </label>

        <p className="settings-hint">
          Policy: when ON, Android keeps the foreground service active while app is backgrounded so sync can continue.
          When OFF, background service is stopped and you should reopen the app for active sync.
        </p>

        <p className="settings-hint">
          Android first-run note: allow notification and nearby-network permission prompts to keep discovery and
          background reliability stable on Android 13+.
        </p>

        <label className="settings-checkbox-row" htmlFor="windowsStartOnLogin">
          <input
            id="windowsStartOnLogin"
            type="checkbox"
            checked={windowsStartOnLogin}
            onChange={(event) => onWindowsStartOnLoginChange(event.target.checked)}
          />
          <span>Start ClipSync on desktop login</span>
        </label>

        <p className="settings-hint">
          Windows/Linux desktop only: when enabled, ClipSync is configured to launch automatically at user sign-in.
        </p>

        <label className="settings-checkbox-row" htmlFor="devModeEnabled">
          <input
            id="devModeEnabled"
            type="checkbox"
            checked={devModeEnabled}
            onChange={(event) => onDevModeEnabledChange(event.target.checked)}
          />
          <span>Developer Mode</span>
        </label>

        <p className="settings-hint">
          Enabling this reveals manual testing tools, advanced diagnostics, and validation tabs.
        </p>

        <button onClick={onSaveSettings} className="settings-save-btn">
          Save Settings
        </button>
        {saveMessage ? <p className="settings-hint">{saveMessage}</p> : null}
      </div>
    </div>
  );
}
