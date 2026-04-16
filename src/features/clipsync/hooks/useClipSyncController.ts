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
import type { AppTab, NativeBridgeStats, SyncStatus, ValidationCase, ValidationResult } from '../../../shared/types/clipsync';

const VALIDATION_STORAGE_KEY = 'clipsync_validation_matrix_v1';

function estimateBase64Bytes(base64: string): number {
  const trimmed = base64.trim();
  if (!trimmed) {
    return 0;
  }

  const padding = trimmed.endsWith('==') ? 2 : trimmed.endsWith('=') ? 1 : 0;
  return Math.max(0, Math.floor((trimmed.length * 3) / 4) - padding);
}

const DEFAULT_VALIDATION_CASES: ValidationCase[] = [
  {
    id: 'discovery_wifi_hotspot',
    title: 'Discovery on Wi-Fi and hotspot',
    description: 'Both peers discover each other over Wi-Fi and hotspot with fallback behavior.',
    result: 'not-run',
    notes: '',
    last_run_at: '',
  },
  {
    id: 'pairing_gate',
    title: 'Pairing gate enforcement',
    description: 'Sync remains blocked until matching 4-digit pairing code is verified on both sides.',
    result: 'not-run',
    notes: '',
    last_run_at: '',
  },
  {
    id: 'android_runtime_permissions',
    title: 'Android runtime permissions',
    description: 'First-run permissions for notifications and nearby-network access are requested and user outcome is validated.',
    result: 'not-run',
    notes: '',
    last_run_at: '',
  },
  {
    id: 'bidirectional_text_image',
    title: 'Bidirectional text and manual image sync',
    description: 'Text and manual image payloads flow both directions with counters and previews updating.',
    result: 'not-run',
    notes: '',
    last_run_at: '',
  },
  {
    id: 'image_uri_supported_apps',
    title: 'URI image capture on supported apps',
    description: 'Apps that expose clipboard image URIs are captured and synced when within configured size limit.',
    result: 'not-run',
    notes: '',
    last_run_at: '',
  },
  {
    id: 'image_unsupported_apps',
    title: 'Unsupported image app fallback behavior',
    description: 'Apps without shareable clipboard image URI do not crash sync and manual image picker path remains usable.',
    result: 'not-run',
    notes: '',
    last_run_at: '',
  },
  {
    id: 'background_continuity_android',
    title: 'Android background continuity',
    description: 'Background mode keeps continuity and expected foreground-service behavior under policy.',
    result: 'not-run',
    notes: '',
    last_run_at: '',
  },
  {
    id: 'reconnect_recovery',
    title: 'Disconnect and recovery',
    description: 'Forced disconnect recovers without restarting apps, with transport status returning to authenticated.',
    result: 'not-run',
    notes: '',
    last_run_at: '',
  },
  {
    id: 'reboot_doze_network_switch',
    title: 'Reboot, doze, and network switch',
    description: 'Behavior remains stable across reboot, doze/background restrictions, and network transitions.',
    result: 'not-run',
    notes: '',
    last_run_at: '',
  },
];

function loadValidationCases(): ValidationCase[] {
  try {
    const raw = window.localStorage.getItem(VALIDATION_STORAGE_KEY);
    if (!raw) {
      return DEFAULT_VALIDATION_CASES;
    }

    const parsed = JSON.parse(raw) as ValidationCase[];
    if (!Array.isArray(parsed) || parsed.length === 0) {
      return DEFAULT_VALIDATION_CASES;
    }

    const byId = new Map(parsed.map((item) => [item.id, item]));
    return DEFAULT_VALIDATION_CASES.map((fallback) => {
      const existing = byId.get(fallback.id);
      if (!existing) {
        return fallback;
      }
      return {
        ...fallback,
        result: existing.result,
        notes: existing.notes,
        last_run_at: existing.last_run_at,
      };
    });
  } catch {
    return DEFAULT_VALIDATION_CASES;
  }
}

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
  const [nativeBridgeStatus, setNativeBridgeStatus] = useState('No native bridge events yet');
  const [validationCases, setValidationCases] = useState<ValidationCase[]>(() => loadValidationCases());
  const [nativeBridgeStats, setNativeBridgeStats] = useState<NativeBridgeStats>({
    captured_text: 0,
    captured_image: 0,
    sent_text: 0,
    sent_image: 0,
    skipped: 0,
    failed: 0,
    malformed: 0,
    last_source: 'n/a',
    last_type: 'unknown',
  });

  const lastClipboardTextRef = useRef('');
  const lastNativeEventSignatureRef = useRef('');
  const previousStatusRef = useRef<SyncStatus>('searching');
  const previousDevicesRef = useRef<string[]>([]);
  const previousPairedRef = useRef(false);

  const syncAndroidBackgroundPolicy = (enabled: boolean) => {
    try {
      const bridge = (window as Window & {
        ClipSyncAndroidPolicy?: { setBackgroundModeEnabled: (value: boolean) => void };
      }).ClipSyncAndroidPolicy;
      bridge?.setBackgroundModeEnabled(enabled);
      uiLog('INFO', 'ANDROID_POLICY_SYNC', `background_mode_enabled=${enabled}`);
    } catch (error) {
      uiLog('FAILED', 'ANDROID_POLICY_SYNC', String(error));
    }
  };

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
    try {
      window.localStorage.setItem(VALIDATION_STORAGE_KEY, JSON.stringify(validationCases));
    } catch {
      uiLog('FAILED', 'VALIDATION_PERSIST', 'unable to persist validation matrix');
    }
  }, [validationCases]);

  useEffect(() => {
    syncAndroidBackgroundPolicy(backgroundModeEnabled);
  }, [backgroundModeEnabled]);

  useEffect(() => {
    const onNativeClipboard = (event: Event) => {
      const customEvent = event as CustomEvent<{
        type?: 'text' | 'image';
        text?: string;
        source?: string;
        mimeType?: string;
        imageBase64?: string;
      }>;
      const nativeType = customEvent.detail?.type ?? 'text';
      const text = customEvent.detail?.text?.trim() ?? '';
      const source = customEvent.detail?.source ?? 'native';
      const mimeType = customEvent.detail?.mimeType ?? '';
      const imageBase64 = customEvent.detail?.imageBase64 ?? '';

      if (nativeType !== 'text' && nativeType !== 'image') {
        setNativeBridgeStatus(`Malformed native event: unknown type from ${source}`);
        setNativeBridgeStats((prev) => ({
          ...prev,
          malformed: prev.malformed + 1,
          failed: prev.failed + 1,
          last_source: source,
          last_type: 'unknown',
        }));
        uiLog('FAILED', 'NATIVE_EVENT_INVALID', `unknown type source=${source}`);
        return;
      }

      const signature = nativeType === 'image'
        ? `image:${mimeType}:${imageBase64.length}:${imageBase64.slice(0, 120)}`
        : `text:${text}`;
      if (signature === lastNativeEventSignatureRef.current) {
        return;
      }
      lastNativeEventSignatureRef.current = signature;

      if (nativeType === 'image') {
        if (!mimeType.startsWith('image/') || !imageBase64) {
          setNativeBridgeStatus(`Malformed native image event from ${source}`);
          setNativeBridgeStats((prev) => ({
            ...prev,
            malformed: prev.malformed + 1,
            failed: prev.failed + 1,
            last_source: source,
            last_type: 'image',
          }));
          uiLog('FAILED', 'NATIVE_IMAGE_INVALID', `source=${source} mime=${mimeType} bytes(base64)=${imageBase64.length}`);
          return;
        }

        const imageBytes = estimateBase64Bytes(imageBase64);
        const maxBytes = maxImageSizeKb * 1024;
        if (imageBytes <= 0) {
          setNativeBridgeStatus(`Malformed native image event from ${source}`);
          setNativeBridgeStats((prev) => ({
            ...prev,
            malformed: prev.malformed + 1,
            failed: prev.failed + 1,
            last_source: source,
            last_type: 'image',
          }));
          uiLog('FAILED', 'NATIVE_IMAGE_INVALID', `source=${source} unable_to_estimate_size=true`);
          return;
        }

        if (imageBytes > maxBytes) {
          setNativeBridgeStatus(
            `Captured image from ${source} but skipped (size ${Math.round(imageBytes / 1024)} KB > limit ${maxImageSizeKb} KB)`,
          );
          setNativeBridgeStats((prev) => ({
            ...prev,
            skipped: prev.skipped + 1,
            last_source: source,
            last_type: 'image',
          }));
          uiLog('INFO', 'NATIVE_IMAGE_SKIPPED_SIZE', `source=${source} size=${imageBytes} limit=${maxBytes}`);
          return;
        }

        setNativeBridgeStatus(`Captured image from ${source}: ${mimeType} bytes(base64)=${imageBase64.length}`);
        setNativeBridgeStats((prev) => ({
          ...prev,
          captured_image: prev.captured_image + 1,
          last_source: source,
          last_type: 'image',
        }));

        if (!paired || !syncEnabled) {
          setNativeBridgeStatus(`Captured image from ${source} but skipped (paired=${paired}, sync=${syncEnabled})`);
          setNativeBridgeStats((prev) => ({
            ...prev,
            skipped: prev.skipped + 1,
            last_source: source,
            last_type: 'image',
          }));
          uiLog('INFO', 'NATIVE_IMAGE_SKIPPED', `source=${source} paired=${paired} sync=${syncEnabled}`);
          return;
        }

        void (async () => {
          try {
            await pushLocalImagePayload(imageBase64, mimeType);
            setNativeBridgeStatus(
              `Captured image from ${source} and sent to peers (${mimeType}, bytes(base64)=${imageBase64.length})`,
            );
            setNativeBridgeStats((prev) => ({
              ...prev,
              sent_image: prev.sent_image + 1,
              last_source: source,
              last_type: 'image',
            }));
            uiLog('SUCCESS', 'IMAGE_SENT_NATIVE', `source=${source} mime=${mimeType} bytes(base64)=${imageBase64.length}`);
          } catch (error) {
            setNativeBridgeStatus(`Captured image from ${source} but send failed`);
            setNativeBridgeStats((prev) => ({
              ...prev,
              failed: prev.failed + 1,
              last_source: source,
              last_type: 'image',
            }));
            uiLog('FAILED', 'IMAGE_SENT_NATIVE', String(error));
          }
        })();
        return;
      }

      if (!text) {
        setNativeBridgeStatus(`Malformed native text event from ${source}`);
        setNativeBridgeStats((prev) => ({
          ...prev,
          malformed: prev.malformed + 1,
          failed: prev.failed + 1,
          last_source: source,
          last_type: 'text',
        }));
        uiLog('FAILED', 'NATIVE_TEXT_INVALID', `source=${source} empty text payload`);
        return;
      }
      lastClipboardTextRef.current = text;
      setNativeBridgeStatus(`Captured from ${source}: ${text.slice(0, 80)}${text.length > 80 ? '...' : ''}`);
      setNativeBridgeStats((prev) => ({
        ...prev,
        captured_text: prev.captured_text + 1,
        last_source: source,
        last_type: 'text',
      }));

      if (!paired || !syncEnabled) {
        setNativeBridgeStatus(`Captured from ${source} but skipped (paired=${paired}, sync=${syncEnabled})`);
        setNativeBridgeStats((prev) => ({
          ...prev,
          skipped: prev.skipped + 1,
          last_source: source,
          last_type: 'text',
        }));
        uiLog('INFO', 'NATIVE_CLIPBOARD_SKIPPED', `source=${source} paired=${paired} sync=${syncEnabled}`);
        return;
      }

      void (async () => {
        try {
          await pushLocalTextClipboard(text);
          setNativeBridgeStatus(`Captured from ${source} and sent to peers (len=${text.length})`);
          setNativeBridgeStats((prev) => ({
            ...prev,
            sent_text: prev.sent_text + 1,
            last_source: source,
            last_type: 'text',
          }));
          uiLog('SUCCESS', 'TEXT_SENT_NATIVE', `source=${source} len=${text.length}`);
        } catch (error) {
          setNativeBridgeStatus(`Captured from ${source} but send failed`);
          setNativeBridgeStats((prev) => ({
            ...prev,
            failed: prev.failed + 1,
            last_source: source,
            last_type: 'text',
          }));
          uiLog('FAILED', 'TEXT_SENT_NATIVE', String(error));
        }
      })();
    };

    window.addEventListener('clipsync-native-clipboard', onNativeClipboard as EventListener);
    return () => {
      window.removeEventListener('clipsync-native-clipboard', onNativeClipboard as EventListener);
    };
  }, [paired, syncEnabled, maxImageSizeKb]);

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
      syncAndroidBackgroundPolicy(backgroundModeEnabled);
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

    if (!file.type.startsWith('image/')) {
      setSyncMessage('Selected file is not a supported image.');
      uiLog('FAILED', 'IMAGE_PICKED', `unsupported mime=${file.type || 'unknown'}`);
      return;
    }

    const maxBytes = maxImageSizeKb * 1024;
    if (file.size > maxBytes) {
      setManualImagePreview('');
      event.target.value = '';
      setSyncMessage(`Image is too large. Current limit is ${maxImageSizeKb} KB.`);
      uiLog('FAILED', 'IMAGE_PICKED', `size=${file.size} exceeds_limit=${maxBytes}`);
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

    const imageBytes = estimateBase64Bytes(base64);
    const maxBytes = maxImageSizeKb * 1024;
    if (imageBytes > maxBytes) {
      setSyncMessage(`Image is too large to sync. Current limit is ${maxImageSizeKb} KB.`);
      uiLog('FAILED', 'IMAGE_SENT_MANUAL', `size=${imageBytes} exceeds_limit=${maxBytes}`);
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

  const onValidationResultChange = (id: string, result: ValidationResult) => {
    const now = new Date().toISOString();
    setValidationCases((prev) => prev.map((item) => {
      if (item.id !== id) {
        return item;
      }
      return {
        ...item,
        result,
        last_run_at: now,
      };
    }));
    uiLog('INFO', 'VALIDATION_RESULT', `${id}=${result}`);
  };

  const onValidationNotesChange = (id: string, notes: string) => {
    setValidationCases((prev) => prev.map((item) => {
      if (item.id !== id) {
        return item;
      }
      return {
        ...item,
        notes,
      };
    }));
  };

  const onResetValidationMatrix = () => {
    setValidationCases(DEFAULT_VALIDATION_CASES);
    uiLog('INFO', 'VALIDATION_RESET');
  };

  const onExportValidationReport = () => {
    const report = {
      exported_at: new Date().toISOString(),
      summary: {
        pass: validationCases.filter((item) => item.result === 'pass').length,
        fail: validationCases.filter((item) => item.result === 'fail').length,
        not_run: validationCases.filter((item) => item.result === 'not-run').length,
      },
      cases: validationCases,
    };

    try {
      const blob = new Blob([JSON.stringify(report, null, 2)], { type: 'application/json' });
      const url = URL.createObjectURL(blob);
      const anchor = document.createElement('a');
      anchor.href = url;
      anchor.download = `clipsync-validation-report-${Date.now()}.json`;
      anchor.click();
      URL.revokeObjectURL(url);
      uiLog('SUCCESS', 'VALIDATION_EXPORT');
      setSyncMessage('Validation report exported as JSON.');
    } catch (error) {
      uiLog('FAILED', 'VALIDATION_EXPORT', String(error));
      setSyncMessage('Validation report export failed.');
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
    nativeBridgeStatus,
    nativeBridgeStats,
    validationCases,
    manualImagePreview,
    remoteImagePreview,
    onToggleSync,
    onSaveSettings,
    onUnlockSync,
    onManualSync,
    onPickManualImage,
    onManualImageSync,
    onValidationResultChange,
    onValidationNotesChange,
    onResetValidationMatrix,
    onExportValidationReport,
  };
}
