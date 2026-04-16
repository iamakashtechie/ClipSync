import type { RuntimeHealth, SyncStats } from '../types/clipsync';

export const DEFAULT_RUNTIME_HEALTH: RuntimeHealth = {
  is_app_foreground: true,
  visibility_report_age_ms: 0,
  background_mode_enabled: true,
  last_auth_age_ms: Number.MAX_SAFE_INTEGER,
  stale_peers_pruned: 0,
  authenticated_peer_count: 0,
};

export const DEFAULT_SYNC_STATS: SyncStats = {
  sent: 0,
  received: 0,
  dropped: 0,
  stale_rejected: 0,
};
