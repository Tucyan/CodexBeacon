type Quota = { used: number | null; limit: number | null } | null;
export function quotaLabel(quota: Quota): string {
  if (!quota || quota.used === null || quota.limit === null) return 'Unavailable';
  return `${quota.used} / ${quota.limit}`;
}

export function providerStatusLabel(state: string): string {
  const labels: Record<string, string> = { idle: 'Idle', loading: 'Loading', ready: 'Ready', offline: 'Offline', error: 'Error', unavailable: 'Unavailable' };
  return labels[state] ?? 'Unknown';
}

export type RateWindow = { windowDurationMins?: number; usedPercent?: number; resetsAt?: number } | null | undefined;

export function remainingPercent(window: RateWindow): number | null {
  if (window?.usedPercent === undefined || !Number.isFinite(window.usedPercent)) return null;
  return Math.max(0, Math.min(100, 100 - window.usedPercent));
}

export function durationLabel(minutes: number | undefined): string {
  if (!Number.isFinite(minutes)) return 'Unknown window';
  if ((minutes ?? 0) >= 60 && (minutes ?? 0) % 60 === 0) return `${(minutes ?? 0) / 60}h window`;
  return `${minutes}m window`;
}

export function localResetLabel(epochSeconds: number | undefined): string {
  if (!Number.isFinite(epochSeconds)) return 'Reset unavailable';
  return new Date((epochSeconds ?? 0) * 1000).toLocaleString();
}
