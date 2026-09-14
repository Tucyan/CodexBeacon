import { useState } from 'react';
import type { AppSettings, Snapshot, ThemeConfig } from '../types/index';
import { dashboardStore } from '../stores/dashboard';
import { Appearance } from './Appearance';
import { providerStatusLabel } from '../utils/provider';
import { WIDGET_REGISTRY } from '../widgets/registry';
import { BackupSettings } from './BackupSettings';
import { AutostartSetting } from './AutostartSetting';

type Tab = 'appearance' | 'widgets' | 'behavior' | 'data' | 'backup';
const tabs: Array<[Tab, string]> = [['appearance', 'Appearance'], ['widgets', 'Widgets'], ['behavior', 'Behavior'], ['data', 'Data'], ['backup', '备份']];

export function SettingsPage({ snapshot }: { snapshot: Snapshot }) {
  const [tab, setTab] = useState<Tab>('appearance');
  const settings = snapshot.settings;
  const save = (next: AppSettings) => void dashboardStore.send({ type: 'settings', settings: next });
  const saveTheme = (patch: Partial<ThemeConfig>) => save({ ...settings, theme: { ...settings.theme, ...patch } });
  return <main className="settings-page">
    <header className="settings-header"><div><span className="eyebrow">Codex Beacon</span><h1>Settings</h1></div></header>
    <nav className="settings-tabs" aria-label="Settings sections">{tabs.map(([id, label]) => <button key={id} className={tab === id ? 'active' : ''} onClick={() => setTab(id)}>{label}</button>)}</nav>
    <section className="settings-content">
      {tab === 'appearance' && <Appearance settings={settings} onTheme={saveTheme} />}
      {tab === 'widgets' && <Widgets snapshot={snapshot} />}
      {tab === 'behavior' && <Behavior settings={settings} save={save} />}
      {tab === 'data' && <Data snapshot={snapshot} />}
      {tab === 'backup' && <BackupSettings settings={settings} save={save} />}
    </section>
  </main>;
}

function Widgets({ snapshot }: { snapshot: Snapshot }) {
  const update = (id: string, patch: { enabled?: boolean; scaleMode?: 'inherit' | 'custom'; scale?: number }) => void dashboardStore.send({ type: 'widget', id, ...patch });
  return <div className="settings-card"><h2>Widgets</h2>{WIDGET_REGISTRY.map((entry) => { const widget = snapshot.widgets.find((item) => item.id === entry.id); if (!widget) return null; return <div className="widget-setting" key={entry.id}><div><strong>{entry.label}</strong><span className="muted">{widget.visibleWanted ? 'Visible' : 'Hidden'}</span></div><label><input type="checkbox" checked={widget.enabled} onChange={(e) => update(widget.id, { enabled: e.target.checked })} /> Enabled</label><button onClick={() => void dashboardStore.send({ type: widget.visibleWanted ? 'hide' : 'show', id: widget.id })}>{widget.visibleWanted ? 'Hide' : 'Show'}</button><button onClick={() => void dashboardStore.send({ type: 'reset-layout', id: widget.id })}>Reset layout</button><label><select value={widget.scaleMode} onChange={(e) => update(widget.id, { scaleMode: e.target.value as 'inherit' | 'custom' })}><option value="inherit">Global scale</option><option value="custom">Custom scale</option></select></label>{widget.scaleMode === 'custom' && <input type="number" min="0.5" max="2" step="0.05" value={widget.scale} aria-label={`${entry.label} scale`} onChange={(e) => update(widget.id, { scale: Number(e.target.value) })} />}</div>; })}</div>;
}

function Behavior({ settings, save }: { settings: AppSettings; save: (settings: AppSettings) => void }) {
  return <div className="settings-card"><h2>Behavior</h2><label className="toggle"><input type="checkbox" checked={settings.layoutLocked} onChange={(e) => save({ ...settings, layoutLocked: e.target.checked })} /> Lock widget layout</label><label>Global scale<input type="range" min="0.5" max="2" step="0.05" value={settings.globalScale} onChange={(e) => save({ ...settings, globalScale: Number(e.target.value) })} /><output>{settings.globalScale.toFixed(2)}×</output></label><label>Show desktop behavior<select value={settings.showDesktopMode} onChange={(e) => save({ ...settings, showDesktopMode: e.target.value as AppSettings['showDesktopMode'] })}><option value="auto">显示桌面时自动出现</option><option value="follow-system">跟随系统</option></select></label><label className="toggle"><input type="checkbox" checked={settings.fadeEnabled} onChange={(e) => save({ ...settings, fadeEnabled: e.target.checked })} /> 启用渐显</label><AutostartSetting /><button className="danger-secondary" onClick={() => void dashboardStore.send({ type: 'reset-layout' })}>Reset layout</button></div>;
}

function Data({ snapshot }: { snapshot: Snapshot }) {
  const provider = (id: 'codex' | 'reset') => snapshot.providers[id];
  const formatSuccess = (value: number | null) => value === null ? 'Never' : new Date(value * 1000).toLocaleString();
  return <div className="settings-card data-card"><h2>Provider data</h2>{(['codex', 'reset'] as const).map((id) => { const item = provider(id); return <section className="data-provider" key={id}><div><h3>{id === 'codex' ? 'Codex Usage' : 'Codex Reset'}</h3><span className={`status-dot ${item.state}`} /> {providerStatusLabel(item.state)}</div><p>Last success: {formatSuccess(item.lastSuccess)}</p>{item.error && <p className="provider-error">Error: {item.error}</p>}<button onClick={() => void dashboardStore.send({ type: 'retry', provider: id })}>Retry</button></section>; })}<p className="muted">Snapshot revision {snapshot.revision}</p></div>;
}
