import { useEffect, useRef, useState, type ChangeEvent } from 'react';
import {
  consumeRemoteImage,
  consumeRemoteText,
  getDiagnostics,
  getSettings,
  getStatus,
  pushLocalImagePayload,
  pushLocalTextClipboard,
  reportAppVisibility,
  saveSettings,
  toggleSync,
  validatePairing,
} from '../../../shared/api/clipsyncApi';
import { DEFAULT_RUNTIME_HEALTH, DEFAULT_SYNC_STATS } from '../../../shared/lib/defaults';
import { uiLog } from '../../../shared/lib/logger';
import type { AppTab, SyncStatus } from '../../../shared/types/clipsync';

export function useClipSyncController() {
  const [currentTab, setCurrentTab] = useState<AppTab>('dashboard');
  const [syncEnabled, setSyncEnabled] = useState(true);
  const [status, setStatus] = useState<SyncStatus>('searching');
  const [devices, setDevices] = useState<string[]>([]);
  const [paired, setPaired] = useState(false);

  const [maxImageSizeKb, setMaxImageSizeKb] = useState(2048);
  const [pairingCode, setPairingCode] = useState('');
  const [deviceNameOverride, setDeviceNameOverride] = useState('');
  const [backgroundModeEnabled, setBackgroundModeEnabled] = useState(true);

  const [unlockCode, setUnlockCode] = useState('');
  const [saveMessage, setSaveMessage] = useState('');
  const [syncMessage, setSyncMessage] = useState('');

  const [peerTransport, setPeerTransport] = useState<Record<string, string>>({});
  const [syncStats, setSyncStats] = useState(DEFAULT_SYNC_STATS);
  const [diagnostics, setDiagnostics] = useState<string[]>([]);
  const [runtimeHealth, setRuntimeHealth] = useState(DEFAULT_RUNTIME_HEALTH);

  const [manualSyncText, setManualSyncText] = useState('');
  const [remoteTextPreview, setRemoteTextPreview] = useState('');
  const [manualImagePreview, setManualImagePreview] = useState('');
  const [manualImageMime, setManualImageMime] = useState('image/png');
  const [remoteImagePreview, setRemoteImagePreview] = useState('');

  const lastClipboardTextRef = useRef('');
  const previousStatusRef = useRef<SyncStatus>('searching');
  const previousDevicesRef = useRef<string[]>([]);
  const previousPairedRef = useRef(false);

  useEffect(() => {
    const fetchStatus = async () => {
      try {
        const res = await getStatus();
        setStatus(res.status);
        setDevices(res.devices);
        setSyncEnabled(res.sync_enabled);
        setPaired(res.paired);
        setRuntimeHealth(res.runtime ?? DEFAULT_RUNTIME_HEALTH);
        setPeerTransport(res.peer_transport ?? {});
        setSyncStats(res.sync_stats ?? DEFAULT_SYNC_STATS);

        if (previousStatusRef.current !== res.status) {
          uiLog('INFO', 'STATUS_CHANGED', `${previousStatusRef.current} -> ${res.status}`);
          previousStatusRef.current = res.status;
        }

        const nextDevices = res.devices ?? [];
        const prevDevices = previousDevicesRef.current;
        if (nextDevices.join('|') !== prevDevices.join('|')) {
          uiLog('INFO', 'DEVICES_UPDATED', `${prevDevices.length} -> ${nextDevices.length}`);
          previousDevicesRef.current = nextDevices;
        }

        if (previousPairedRef.current !== res.paired) {
          uiLog('INFO', 'PAIR_STATE_CHANGED', `${previousPairedRef.current} -> ${res.paired}`);
          previousPairedRef.current = res.paired;
        }
      } catch (error) {
        uiLog('FAILED', 'GET_STATUS', String(error));
      }
    };

    const fetchDiagnostics = async () => {
      try {
        const events = await getDiagnostics();
        setDiagnostics(events.slice(-8));
      } catch (error) {
        uiLog('FAILED', 'GET_DIAGNOSTICS', String(error));
      }
    };

    const fetchSettings = async () => {
      try {
        const settings = await getSettings();
        setMaxImageSizeKb(settings.max_image_size_kb);
        setPairingCode(settings.pairing_code);
        setDeviceNameOverride(settings.device_name_override ?? '');
        setBackgroundModeEnabled(settings.background_mode_enabled ?? true);
        uiLog('SUCCESS', 'SETTINGS_LOADED');
      } catch (error) {
        uiLog('FAILED', 'GET_SETTINGS', String(error));
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
    const report = async (isForeground: boolean) => {
      try {
        await reportAppVisibility(isForeground);
        uiLog('INFO', 'REPORT_APP_VISIBILITY', isForeground ? 'foreground' : 'background');
      } catch (error) {
        uiLog('FAILED', 'REPORT_APP_VISIBILITY', String(error));
      }
    };

    const onVisibility = () => {
      report(!document.hidden);
    };

    const onFocus = () => report(true);
    const onBlur = () => report(false);

    document.addEventListener('visibilitychange', onVisibility);
    window.addEventListener('focus', onFocus);
    window.addEventListener('blur', onBlur);
    report(!document.hidden);

    return () => {
      document.removeEventListener('visibilitychange', onVisibility);
      window.removeEventListener('focus', onFocus);
      window.removeEventListener('blur', onBlur);
    };
  }, []);

  useEffect(() => {
    const interval = window.setInterval(async () => {
      if (!paired || !syncEnabled) {
        return;
      }

      try {
        const remote = await consumeRemoteText();
        if (remote) {
          setRemoteTextPreview(remote);
          uiLog('SUCCESS', 'TEXT_RECEIVED', `len=${remote.length}`);
          try {
            await navigator.clipboard.writeText(remote);
            uiLog('SUCCESS', 'LOCAL_CLIPBOARD_WRITE', 'remote text applied');
          } catch {
            uiLog('FAILED', 'LOCAL_CLIPBOARD_WRITE', 'clipboard API write failed');
          }
        }
      } catch (error) {
        uiLog('FAILED', 'CONSUME_REMOTE_TEXT', String(error));
      }

      try {
        const remoteImage = await consumeRemoteImage();
        if (remoteImage && remoteImage.image_base64) {
          setRemoteImagePreview(`data:${remoteImage.mime_type};base64,${remoteImage.image_base64}`);
          uiLog('SUCCESS', 'IMAGE_RECEIVED', `${remoteImage.mime_type} bytes(base64)=${remoteImage.image_base64.length}`);
        }
      } catch (error) {
        uiLog('FAILED', 'CONSUME_REMOTE_IMAGE', String(error));
      }

      try {
        const localText = await navigator.clipboard.readText();
        if (localText && localText !== lastClipboardTextRef.current) {
          lastClipboardTextRef.current = localText;
          await pushLocalTextClipboard(localText);
          uiLog('SUCCESS', 'TEXT_SENT_AUTO', `len=${localText.length}`);
        }
      } catch {
        uiLog('FAILED', 'TEXT_SENT_AUTO', 'clipboard read or push failed');
      }
    }, 1200);

    return () => window.clearInterval(interval);
  }, [paired, syncEnabled]);

  const onToggleSync = async () => {
    const next = !syncEnabled;
    try {
      await toggleSync(next);
      setSyncEnabled(next);
      setSyncMessage('');
      uiLog('SUCCESS', 'TOGGLE_SYNC', next ? 'enabled' : 'disabled');
    } catch (error) {
      uiLog('FAILED', 'TOGGLE_SYNC', String(error));
      setSyncMessage(String(error));
    }
  };

  const onSaveSettings = async () => {
    if (!/^\d{4}$/.test(pairingCode)) {
      setSaveMessage('Pairing code must be exactly 4 digits.');
      uiLog('FAILED', 'SAVE_SETTINGS', 'invalid pairing code format');
      return;
    }

    try {
      await saveSettings({
        maxImageSizeKb,
        pairingCode,
        deviceNameOverride,
        backgroundModeEnabled,
      });
      setSaveMessage('Settings saved. Device name update may require app restart for discovery name refresh.');
      setPaired(false);
      setSyncEnabled(false);
      uiLog('SUCCESS', 'SAVE_SETTINGS', `max_image_size_kb=${maxImageSizeKb} bg_mode=${backgroundModeEnabled}`);
    } catch (error) {
      uiLog('FAILED', 'SAVE_SETTINGS', String(error));
      setSaveMessage('Failed to save settings.');
    }
  };

  const onUnlockSync = async () => {
    if (!/^\d{4}$/.test(unlockCode)) {
      setSyncMessage('Enter your 4-digit pairing code to unlock sync.');
      uiLog('FAILED', 'VALIDATE_PAIRING', 'unlock code is not 4 digits');
      return;
    }

    try {
      const ok = await validatePairing(unlockCode);
      if (ok) {
        setPaired(true);
        setSyncMessage('Pairing verified. You can enable sync now.');
        uiLog('SUCCESS', 'VALIDATE_PAIRING', 'pairing verified');
      } else {
        setPaired(false);
        setSyncEnabled(false);
        setSyncMessage('Invalid pairing code.');
        uiLog('FAILED', 'VALIDATE_PAIRING', 'pairing mismatch');
      }
    } catch (error) {
      uiLog('FAILED', 'VALIDATE_PAIRING', String(error));
      setSyncMessage('Unable to verify pairing code right now.');
    }
  };

  const onManualSync = async () => {
    if (!manualSyncText.trim()) {
      setSyncMessage('Enter text to send.');
      uiLog('FAILED', 'TEXT_SENT_MANUAL', 'empty payload');
      return;
    }

    try {
      await pushLocalTextClipboard(manualSyncText);
      setSyncMessage('Text sent to authenticated peers.');
      uiLog('SUCCESS', 'TEXT_SENT_MANUAL', `len=${manualSyncText.length}`);
    } catch (error) {
      uiLog('FAILED', 'TEXT_SENT_MANUAL', String(error));
      setSyncMessage('Manual sync failed.');
    }
  };

  const onPickManualImage = async (event: ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (!file) {
      return;
    }

    setManualImageMime(file.type || 'image/png');
    uiLog('INFO', 'IMAGE_PICKED', `${file.type || 'image/png'} bytes=${file.size}`);

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
      uiLog('FAILED', 'IMAGE_SENT_MANUAL', 'image not selected');
      return;
    }

    const base64 = manualImagePreview.split(',')[1] ?? '';
    if (!base64) {
      setSyncMessage('Invalid image payload.');
      uiLog('FAILED', 'IMAGE_SENT_MANUAL', 'invalid data url payload');
      return;
    }

    try {
      await pushLocalImagePayload(base64, manualImageMime);
      setSyncMessage('Image sent to authenticated peers.');
      uiLog('SUCCESS', 'IMAGE_SENT_MANUAL', `${manualImageMime} bytes(base64)=${base64.length}`);
    } catch (error) {
      uiLog('FAILED', 'IMAGE_SENT_MANUAL', String(error));
      setSyncMessage('Manual image sync failed.');
    }
  };

  return {
    currentTab,
    setCurrentTab,
    syncEnabled,
    status,
    devices,
    paired,
    maxImageSizeKb,
    setMaxImageSizeKb,
    pairingCode,
    setPairingCode,
    deviceNameOverride,
    setDeviceNameOverride,
    backgroundModeEnabled,
    setBackgroundModeEnabled,
    unlockCode,
    setUnlockCode,
    saveMessage,
    syncMessage,
    peerTransport,
    syncStats,
    diagnostics,
    runtimeHealth,
    manualSyncText,
    setManualSyncText,
    remoteTextPreview,
    manualImagePreview,
    remoteImagePreview,
    onToggleSync,
    onSaveSettings,
    onUnlockSync,
    onManualSync,
    onPickManualImage,
    onManualImageSync,
  };
}
