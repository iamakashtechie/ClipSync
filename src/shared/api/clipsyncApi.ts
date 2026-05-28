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

export async function pushLocalTextClipboard(text: string): Promise<void> {
  await invoke('push_local_text_clipboard', { text });
}

export async function toggleSync(enabled: boolean): Promise<void> {
  await invoke('toggle_sync', { enabled });
}

export async function saveSettings(payload: {
  maxImageSizeKb: number;
  deviceNameOverride: string;
  backgroundModeEnabled: boolean;
  windowsStartOnLogin: boolean;
  devModeEnabled: boolean;
}): Promise<void> {
  await invoke('save_settings', payload);
}

export async function pushLocalImagePayload(imageBase64: string, mimeType: string): Promise<void> {
  await invoke('push_local_image_payload', {
    imageBase64,
    mimeType,
  });
}

export async function requestConnection(peerName: string): Promise<void> {
  await invoke('request_connection', { peerName });
}

export async function approveConnection(peerName: string): Promise<void> {
  await invoke('approve_connection', { peerName });
}

export async function rejectConnection(peerName: string): Promise<void> {
  await invoke('reject_connection', { peerName });
}

export async function readClipboardText(): Promise<string | null> {
  return invoke<string | null>('read_clipboard_text');
}

export async function writeClipboardText(text: string): Promise<void> {
  await invoke('write_clipboard_text', { text });
}
