import { useEffect, useMemo, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { CSSProperties } from 'react';
import { subscribeToAppearance, widgetReady } from '../runtime/tauri';
import { dashboardStore, useDashboard, useDashboardError } from '../stores/dashboard';
import { routeFromSearch } from '../utils/query';
import { effectiveScale } from '../utils/scale';
import { themeVariables } from '../utils/theme';
import { widgetEntry } from '../widgets/registry';
import { SettingsPage } from '../settings/SettingsPage';
import './styles.css';

export function App() {
  const snapshot = useDashboard();
  const runtimeError = useDashboardError();
  const [appearance, setAppearance] = useState<'still' | 'animate' | 'prepare-hidden' | 'prepare-visible'>('still');
  const shellRef = useRef<HTMLDivElement>(null);
  const widgetReadyRef = useRef(false);
  const appearanceTimerRef = useRef<number | undefined>(undefined);
  const appearanceTokenRef = useRef(-1);
  const route = useMemo(() => routeFromSearch(window.location.search), []);
  useEffect(() => { let unlisten: (() => void) | undefined; let cancelled = false; const setup = async () => { try { const cleanup = await subscribeToAppearance((event) => { if (event.revision < appearanceTokenRef.current) return; appearanceTokenRef.current = event.revision; if (appearanceTimerRef.current !== undefined) window.clearTimeout(appearanceTimerRef.current); if (shellRef.current) { shellRef.current.style.transition = ''; shellRef.current.style.opacity = ''; } setAppearance(event.animate ? 'animate' : 'still'); if (event.animate) appearanceTimerRef.current = window.setTimeout(() => { appearanceTimerRef.current = undefined; if (appearanceTokenRef.current === event.revision) setAppearance('still'); }, 250); }, (event) => { if (event.revision < appearanceTokenRef.current) return; appearanceTokenRef.current = event.revision; if (appearanceTimerRef.current !== undefined) window.clearTimeout(appearanceTimerRef.current); if (shellRef.current) { shellRef.current.style.transition = 'none'; shellRef.current.style.opacity = event.animate ? '0' : '1'; } setAppearance(event.animate ? 'prepare-hidden' : 'prepare-visible'); }); if (cancelled) cleanup(); else { unlisten = cleanup; await dashboardStore.start(); } } catch { dashboardStore.reportError('Appearance bridge unavailable. Reload to retry.'); } }; void setup(); return () => { cancelled = true; unlisten?.(); if (appearanceTimerRef.current !== undefined) window.clearTimeout(appearanceTimerRef.current); dashboardStore.stop(); }; }, []);
  useEffect(() => { if (snapshot && !widgetReadyRef.current) { widgetReadyRef.current = true; void widgetReady().catch(() => undefined); } }, [snapshot]);
  useEffect(() => {
    if (snapshot && (window as Window & { __DASHBOARD_SMOKE__?: boolean }).__DASHBOARD_SMOKE__) {
      void invoke('renderer_audit', { revision: snapshot.revision, widgetIds: snapshot.widgets.map(item => item.id), renderedId: shellRef.current?.querySelector('[data-widget-id]')?.getAttribute('data-widget-id') ?? null });
    }
  }, [snapshot]);
  if (!snapshot) return <div className="loading-screen"><span className="spinner" />{runtimeError ?? 'Loading dashboard…'}{runtimeError && <button onClick={() => window.location.reload()}>Retry</button>}</div>;
  const theme = snapshot.settings.theme;
  const widget = route.type === 'widget' ? snapshot.widgets.find((item) => item.id === route.id) : undefined;
  const renderScale = route.type === 'settings' ? 1 : widget ? effectiveScale(snapshot.settings.globalScale, widget.scaleMode, widget.scale) : snapshot.settings.globalScale;
  const style = { ...themeVariables(theme), '--render-scale': renderScale } as CSSProperties;
  return <div ref={shellRef} className={`app-shell ${route.type === 'settings' ? 'settings-shell' : ''} ${appearance === 'animate' ? 'appearance-animate' : ''} ${appearance === 'prepare-hidden' ? 'appearance-prepare-hidden' : ''} ${appearance === 'prepare-visible' ? 'appearance-prepare-visible' : ''}`} style={style}>
    {runtimeError && <div className="error-banner" role="alert">{runtimeError}<button onClick={() => dashboardStore.retrySaves()}>Retry saves</button><button onClick={() => dashboardStore.clearError()}>Dismiss</button></div>}
    {route.type === 'settings' ? <SettingsPage snapshot={snapshot} /> : route.type === 'widget' ? <WidgetView snapshot={snapshot} widget={widget} /> : <Home snapshot={snapshot} />}
  </div>;
}

function WidgetView({ snapshot, widget }: { snapshot: NonNullable<ReturnType<typeof useDashboard>>; widget: typeof snapshot.widgets[number] | undefined }) {
  if (!widget) return <div className="empty-state"><h1>Widget unavailable</h1><p>This widget does not exist.</p></div>;
  const entry = widgetEntry(widget.id);
  if (!entry) return <div className="empty-state"><h1>Widget unavailable</h1><p>Unknown widget type.</p></div>;
  const Component = entry.Component;
  return widget.enabled ? <Component widget={widget} snapshot={snapshot} /> : <div className="empty-state"><h1>Widget disabled</h1><p>Enable it in Settings.</p></div>;
}

function Home({ snapshot }: { snapshot: NonNullable<ReturnType<typeof useDashboard>> }) {
  const visible = snapshot.widgets.filter((widget) => widget.enabled && widget.visibleWanted);
  return <div className="empty-state"><span className="eyebrow">Desktop Dashboard</span><h1>{visible.length ? 'Widgets are ready' : 'No widgets enabled'}</h1><p>Open a widget window or use Settings to configure the dashboard.</p></div>;
}
