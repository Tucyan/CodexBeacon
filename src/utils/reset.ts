export type ResetAnnouncement = { id: string; announcedAt: string; resetType: string; sourceType?: string };
export type ResetWatch = { level: string; chancePercent: number | null; forecastWindow: string; observedAt: string; expiresAt: string };
export type ResetData = { latestReset: ResetAnnouncement | null; activeWatch?: ResetWatch | null };

export function announcementLabel(kind: string): string {
  return kind === 'banked' ? '赠送重置次数' : kind === 'regular' ? '额度重置公告' : '重置相关公告';
}
export function elapsedLabel(announcedAt: string, now: number): string {
  const elapsed = now - Date.parse(announcedAt);
  if (!Number.isFinite(elapsed)) return '时间不可用';
  if (elapsed < 0) return '时间待核实';
  if (elapsed < 60_000) return '刚刚';
  if (elapsed < 3_600_000) return `${Math.floor(elapsed / 60_000)} 分钟前`;
  if (elapsed < 86_400_000) return `${Math.floor(elapsed / 3_600_000)} 小时前`;
  return `${Math.floor(elapsed / 86_400_000)} 天前`;
}
export function activeForecast(watch: ResetWatch | null | undefined, now: number): ResetWatch | null {
  if (!watch || !Number.isFinite(Date.parse(watch.expiresAt)) || Date.parse(watch.expiresAt) <= now) return null;
  return watch;
}
