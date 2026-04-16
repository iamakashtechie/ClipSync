import { invoke } from '@tauri-apps/api/core';
import type { IncomingImage, SettingsResponse, StatusResponse } from '../types/clipsync';

export async function getStatus(): Promise<StatusResponse> {
  return invoke<StatusResponse>('get_status');
}

export async function getDiagnostics(): Promise<string[]> {
  return invoke<string[]>('get_diagnostics');
}

export async function getSettings(): Promise<SettingsResponse> {
  return invoke<SettingsResponse>('get_settings');
}

export async function reportAppVisibility(isForeground: boolean): Promise<void> {
  await invoke('report_app_visibility', { isForeground });
}

export async function consumeRemoteText(): Promise<string | null> {
  return invoke<string | null>('consume_remote_text');
}

export async function consumeRemoteImage(): Promise<IncomingImage | null> {
  return invoke<IncomingImage | null>('consume_remote_image');
}

export async function pushLocalTextClipboard(content: string): Promise<void> {
  await invoke('push_local_text_clipboard', { content });
}

export async function toggleSync(enabled: boolean): Promise<void> {
  await invoke('toggle_sync', { enabled });
}

export async function saveSettings(payload: {
  maxImageSizeKb: number;
  pairingCode: string;
  deviceNameOverride: string;
  backgroundModeEnabled: boolean;
}): Promise<void> {
  await invoke('save_settings', payload);
}

export async function validatePairing(code: string): Promise<boolean> {
  return invoke<boolean>('validate_pairing', { code });
}

export async function pushLocalImagePayload(imageBase64: string, mimeType: string): Promise<void> {
  await invoke('push_local_image_payload', {
    imageBase64,
    mimeType,
  });
}
