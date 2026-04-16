import { DashboardView } from '../features/dashboard/DashboardView';
import { useClipSyncController } from '../features/clipsync/hooks/useClipSyncController';
import { AppFooter } from '../features/layout/AppFooter';
import { AppHeader } from '../features/layout/AppHeader';
import { SettingsView } from '../features/settings/SettingsView';
import { ValidationView } from '../features/validation/ValidationView';

export default function AppShell() {
  const controller = useClipSyncController();

  return (
    <div className="min-h-screen bg-gray-950 text-white flex flex-col">
      <AppHeader currentTab={controller.currentTab} onTabChange={controller.setCurrentTab} />

      <div className="flex-1 p-8">
        {controller.currentTab === 'dashboard' ? (
          <DashboardView
            status={controller.status}
            paired={controller.paired}
            unlockCode={controller.unlockCode}
            onUnlockCodeChange={controller.setUnlockCode}
            onUnlockSync={controller.onUnlockSync}
            syncEnabled={controller.syncEnabled}
            onToggleSync={controller.onToggleSync}
            syncMessage={controller.syncMessage}
            devices={controller.devices}
            peerTransport={controller.peerTransport}
            syncStats={controller.syncStats}
            diagnostics={controller.diagnostics}
            runtimeHealth={controller.runtimeHealth}
            manualSyncText={controller.manualSyncText}
            onManualSyncTextChange={controller.setManualSyncText}
            onManualSync={controller.onManualSync}
            remoteTextPreview={controller.remoteTextPreview}
            nativeBridgeStatus={controller.nativeBridgeStatus}
            nativeBridgeStats={controller.nativeBridgeStats}
            onPickManualImage={controller.onPickManualImage}
            manualImagePreview={controller.manualImagePreview}
            onManualImageSync={controller.onManualImageSync}
            remoteImagePreview={controller.remoteImagePreview}
          />
        ) : controller.currentTab === 'validation' ? (
          <ValidationView
            cases={controller.validationCases}
            onResultChange={controller.onValidationResultChange}
            onNotesChange={controller.onValidationNotesChange}
            onExport={controller.onExportValidationReport}
            onReset={controller.onResetValidationMatrix}
          />
        ) : (
          <SettingsView
            maxImageSizeKb={controller.maxImageSizeKb}
            onMaxImageSizeKbChange={controller.setMaxImageSizeKb}
            pairingCode={controller.pairingCode}
            onPairingCodeChange={controller.setPairingCode}
            deviceNameOverride={controller.deviceNameOverride}
            onDeviceNameOverrideChange={controller.setDeviceNameOverride}
            backgroundModeEnabled={controller.backgroundModeEnabled}
            onBackgroundModeEnabledChange={controller.setBackgroundModeEnabled}
            windowsStartOnLogin={controller.windowsStartOnLogin}
            onWindowsStartOnLoginChange={controller.setWindowsStartOnLogin}
            onSaveSettings={controller.onSaveSettings}
            saveMessage={controller.saveMessage}
          />
        )}
      </div>

      <AppFooter />
    </div>
  );
}
