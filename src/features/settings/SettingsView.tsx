type SettingsViewProps = {
  maxImageSizeKb: number;
  onMaxImageSizeKbChange: (value: number) => void;
  pairingCode: string;
  onPairingCodeChange: (value: string) => void;
  deviceNameOverride: string;
  onDeviceNameOverrideChange: (value: string) => void;
  backgroundModeEnabled: boolean;
  onBackgroundModeEnabledChange: (value: boolean) => void;
  onSaveSettings: () => void;
  saveMessage: string;
};

export function SettingsView({
  maxImageSizeKb,
  onMaxImageSizeKbChange,
  pairingCode,
  onPairingCodeChange,
  deviceNameOverride,
  onDeviceNameOverrideChange,
  backgroundModeEnabled,
  onBackgroundModeEnabledChange,
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

        <label className="settings-label" htmlFor="pairingCode">
          Mandatory pairing code (4 digits)
        </label>
        <input
          id="pairingCode"
          type="text"
          maxLength={4}
          value={pairingCode}
          onChange={(event) => onPairingCodeChange(event.target.value.replace(/\D/g, ''))}
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
          <span>Background reliability mode (preview)</span>
        </label>

        <button onClick={onSaveSettings} className="settings-save-btn">
          Save Settings
        </button>
        {saveMessage ? <p className="settings-hint">{saveMessage}</p> : null}
      </div>
    </div>
  );
}
