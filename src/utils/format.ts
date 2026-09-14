export function normalizeHex(value: string): string {
  const raw = value.trim().replace(/^#/, '');
  if (/^[0-9a-f]{6}$/i.test(raw)) return `#${raw.toUpperCase()}`;
  return '#000000';
}

export function withAlpha(value: string, opacity: number): string {
  const hex = normalizeHex(value).slice(1);
  const rgb = [0, 2, 4].map((i) => Number.parseInt(hex.slice(i, i + 2), 16));
  const alpha = Math.max(0, Math.min(1, opacity));
  return `rgba(${rgb.join(', ')}, ${Number(alpha.toFixed(3))})`;
}

export function readableRemaining(seconds: number): string {
  const total = Math.max(0, Math.floor(seconds));
  if (!total) return 'Expired';
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  const secs = total % 60;
  return [hours ? `${hours}h` : '', minutes ? `${minutes}m` : '', `${secs}s`].filter(Boolean).join(' ');
}
