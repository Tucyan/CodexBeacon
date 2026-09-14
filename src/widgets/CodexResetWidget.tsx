import { WidgetFrame } from '../components/WidgetFrame';
import { useEffect, useState } from 'react';
import { dashboardStore } from '../stores/dashboard';
import { providerStatusLabel } from '../utils/provider';
import type { WidgetProps } from './registry';
import { activeForecast, announcementLabel, elapsedLabel, type ResetData } from '../utils/reset';
import './reset.css';

export function CodexResetWidget({ widget, snapshot }: WidgetProps) {
  const provider = snapshot.providers.reset;
  const data = provider.data as ResetData | null;
  const reset = data?.latestReset;
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => { const timer = window.setInterval(() => setNow(Date.now()), 60_000); return () => window.clearInterval(timer); }, []);
  const resetDate = reset ? new Date(reset.announcedAt) : null;
  const validDate = resetDate && Number.isFinite(resetDate.getTime());
  const watch = activeForecast(data?.activeWatch, now);
  return <WidgetFrame widget={widget} locked={snapshot.settings.layoutLocked} title="Codex Reset" onHide={() => void dashboardStore.send({ type: 'hide', id: widget.id })}>
    <div className="reset-details">
      {reset ? <>
        <span className="reset-caption muted">{reset.sourceType === 'observed' ? '最近记录 · 网站观察' : '最近公告'}</span>
        <strong>{announcementLabel(reset.resetType)}</strong>
        <div className="reset-time"><time dateTime={validDate ? resetDate.toISOString() : undefined}>{validDate ? resetDate.toLocaleString() : '时间不可用'}</time><span className="muted">{elapsedLabel(reset.announcedAt, now)}</span></div>
        {reset.resetType === 'banked' && <span className="reset-note muted">需手动使用，不代表额度已立即恢复</span>}
      </> : <span className="muted">{provider.state === 'ready' ? '暂无公告记录' : providerStatusLabel(provider.state)}</span>}
      {watch && <div className="reset-forecast"><span className="reset-caption">预测 · AI 分类，非官方承诺</span><span>{watch.chancePercent == null ? '存在预测信号' : `预测概率 ${watch.chancePercent}%`}{watch.forecastWindow && ` · ${watch.forecastWindow}`}</span></div>}
      {reset && provider.state !== 'ready' && <span className="reset-note muted">{providerStatusLabel(provider.state)}</span>}
      <span className="reset-source muted">codex-resets.com · 非官方追踪</span>
    </div>
  </WidgetFrame>;
}
