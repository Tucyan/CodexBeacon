export function secondsUntil(targetEpochSeconds: number, nowEpochSeconds = Math.floor(Date.now() / 1000)): number {
  return Math.max(0, Math.floor(targetEpochSeconds - nowEpochSeconds));
}

export function formatCountdown(totalSeconds: number): string {
  const total = Math.max(0, Math.floor(totalSeconds));
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  const seconds = total % 60;
  return [hours, minutes, seconds].map((part) => String(part).padStart(2, '0')).join(':');
}
