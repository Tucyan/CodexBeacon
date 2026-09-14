export type ScaleMode = 'inherit' | 'custom';

export function effectiveScale(globalScale: number, mode: ScaleMode, widgetScale: number): number {
  const global = Number.isFinite(globalScale) && globalScale > 0 ? globalScale : 1;
  if (mode === 'inherit') return global;
  const custom = Number.isFinite(widgetScale) && widgetScale > 0 ? widgetScale : 1;
  return global * custom;
}

export function scaledDip(value: number, scale: number): number {
  return Math.round(value * (Number.isFinite(scale) && scale > 0 ? scale : 1));
}
