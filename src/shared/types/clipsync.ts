export type AppTab = 'dashboard' | 'settings';

export type SyncStatus = 'searching' | 'connected';

export type RuntimeHealth = {
  is_app_foreground: boolean;
  visibility_report_age_ms: number;
  background_mode_enabled: boolean;
  last_auth_age_ms: number;
  stale_peers_pruned: number;
  authenticated_peer_count: number;
};

export type SyncStats = {
  sent: number;
  received: number;
  dropped: number;
  stale_rejected: number;
};

export type StatusResponse = {
  status: SyncStatus;
  devices: string[];
  sync_enabled: boolean;
  paired: boolean;
  runtime?: RuntimeHealth;
  peer_transport?: Record<string, string>;
  sync_stats?: SyncStats;
};

export type SettingsResponse = {
  max_image_size_kb: number;
  pairing_code: string;
  device_name_override: string;
  background_mode_enabled: boolean;
};

export type IncomingImage = {
  mime_type: string;
  image_base64: string;
};

export type UiLogLevel = 'INFO' | 'SUCCESS' | 'FAILED';
