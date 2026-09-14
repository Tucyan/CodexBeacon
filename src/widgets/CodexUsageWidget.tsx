import { WidgetFrame } from '../components/WidgetFrame';
import { dashboardStore } from '../stores/dashboard';
import { durationLabel, localResetLabel, remainingPercent, type RateWindow } from '../utils/provider';
import type { WidgetProps } from './registry';

type CodexData = { account?: { planType?: string; authType?: string }; rateLimits?: { buckets?: Array<{ limitId?: string | null; primary?: RateWindow; secondary?: RateWindow; }> | null; resetCreditsAvailableCount?: number | null } | null } | null;

function clampPercent(value: number | undefined): number | null {
  return Number.isFinite(value) ? Math.max(0, Math.min(100, value as number)) : null;
}

function RateWindowCard({ label, window }: { label: string; window: RateWindow }) {
  const used = clampPercent(window?.usedPercent);
  const remaining = remainingPercent(window);
  return <div className="rate-window"><div className="rate-window-heading"><strong>{label}</strong><span>{durationLabel(window?.windowDurationMins)}</span></div>{used === null || remaining === null ? <span className="muted">No quota data</span> : <><div className="rate-progress"><progress max={100} value={remaining} aria-label={`${label} remaining`} /></div><span>{used.toFixed(0)}% used · {remaining.toFixed(0)}% remaining</span><time className="muted">{localResetLabel(window?.resetsAt)}</time></>}</div>;
}

export function CodexUsageWidget({ widget, snapshot }: WidgetProps) {
  const provider = snapshot.providers.codex;
  const data = provider.data as CodexData;
  const buckets = data?.rateLimits?.buckets ?? [];
  const bucket = buckets.find((item) => item.limitId?.toLowerCase() === 'codex') ?? buckets[0];
  return <WidgetFrame widget={widget} locked={snapshot.settings.layoutLocked} title="Codex Usage" bodyClassName="usage-body" onHide={() => void dashboardStore.send({ type: 'hide', id: widget.id })}>
    {data ? <><p>{data.account?.planType ?? 'Plan unavailable'} · {data.account?.authType ?? 'Auth unavailable'}</p>{data.rateLimits?.resetCreditsAvailableCount !== null && data.rateLimits?.resetCreditsAvailableCount !== undefined && <p className="banked-count">Banked reset credits: <strong>{data.rateLimits.resetCreditsAvailableCount}</strong></p>}{bucket ? <div className="rate-list">{buckets.length > 1 && bucket.limitId?.toLowerCase() !== 'codex' && <span className="muted">{bucket.limitId ?? 'Selected rate limit'}</span>}<RateWindowCard label="Primary" window={bucket.primary} /><RateWindowCard label="Secondary" window={bucket.secondary} /></div> : <p className="muted">No quota data</p>}</> : <p className="muted">Quota unavailable{provider.error ? ` (${provider.error})` : ''}</p>}
  </WidgetFrame>;
}
