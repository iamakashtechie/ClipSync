import type { UiLogLevel } from '../types/clipsync';

export function uiLog(level: UiLogLevel, event: string, details?: string): void {
  const prefix = `[ClipSync/UI][${level}] ${event}`;

  if (details) {
    if (level === 'FAILED') {
      console.error(`${prefix} :: ${details}`);
    } else {
      console.log(`${prefix} :: ${details}`);
    }
    return;
  }

  if (level === 'FAILED') {
    console.error(prefix);
  } else {
    console.log(prefix);
  }
}
