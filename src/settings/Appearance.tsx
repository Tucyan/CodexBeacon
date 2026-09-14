import { useEffect, useState, type CSSProperties } from 'react';
import type { AppSettings, ThemeConfig } from '../types/index';
import { normalizeHex } from '../utils/format';
import { THEME_COLOR_ROLES, THEME_PRESETS, applyPreset, matchingPreset, themeVariables } from '../utils/theme';
import './appearance.css';

export function Appearance({ settings, onTheme }: { settings: AppSettings; onTheme: (patch: Partial<ThemeConfig>) => void }) {
  const theme = settings.theme;
  const active = matchingPreset(theme);
  return <div className="settings-grid">
    <section className="settings-card theme-library" aria-labelledby="theme-heading">
      <div className="theme-section-heading"><h2 id="theme-heading">主题</h2><span className="muted" aria-live="polite">当前：{active?.name ?? '自定义'}</span></div>
      <p className="theme-help muted">选择一组配色，再按需微调。切换主题会保留背景和内容透明度。</p>
      {(['dark', 'light'] as const).map(mode => <div className="theme-group" key={mode}>
        <h3>{mode === 'dark' ? '深色' : '浅色'} <span className="muted">4</span></h3>
        <div className="theme-presets">{THEME_PRESETS.filter(preset => preset.mode === mode).map(preset => <button
          type="button" key={preset.id} className="theme-preset" aria-pressed={active?.id === preset.id}
          aria-label={`应用${preset.name}主题`} onClick={() => onTheme(applyPreset(theme, preset))}
          style={themeVariables({ ...theme, ...preset.colors }) as CSSProperties}
        >
          <span className="theme-sample" aria-hidden="true"><span className="theme-sample-title">Aa <span>···</span></span><span className="theme-sample-field">记录一点想法</span><span className="theme-sample-progress"><span /></span></span>
          <span className="theme-preset-name">{preset.name}<span aria-hidden="true">{active?.id === preset.id ? '✓' : ''}</span></span>
          <span className="theme-swatches" aria-hidden="true">{THEME_COLOR_ROLES.map(({ key }) => <i key={key} style={{ backgroundColor: preset.colors[key] }} />)}</span>
        </button>)}</div>
      </div>)}
    </section>
    <section className="settings-card" aria-labelledby="colors-heading"><h2 id="colors-heading">五个基础颜色</h2>
      <p className="theme-help muted">边框自动混色；按钮使用强调色，按钮文字自动匹配。</p>
      {THEME_COLOR_ROLES.map(({ key, label, hint }) => <ColorField key={key} label={label} hint={hint} value={theme[key]} onCommit={value => onTheme({ [key]: value })} />)}
    </section>
    <section className="settings-card"><h2>透明度</h2>
      <label>背景不透明度<input type="range" min="0" max="1" step="0.01" value={theme.backgroundOpacity} onChange={e => onTheme({ backgroundOpacity: Number(e.target.value) })} /><output>{Math.round(theme.backgroundOpacity * 100)}%</output></label>
      <label>内容不透明度<input type="range" min="0" max="1" step="0.01" value={theme.contentOpacity} onChange={e => onTheme({ contentOpacity: Number(e.target.value) })} /><output>{Math.round(theme.contentOpacity * 100)}%</output></label>
    </section>
  </div>;
}

function ColorField({ label, hint, value, onCommit }: { label: string; hint: string; value: string; onCommit: (value: string) => void }) {
  const [draft, setDraft] = useState(value);
  useEffect(() => setDraft(value), [value]);
  const commit = () => {
    if (/^#?[0-9a-f]{6}$/i.test(draft.trim())) onCommit(normalizeHex(draft));
    else setDraft(value);
  };
  return <label className="color-control"><span className="color-caption">{label}<small className="muted">{hint}</small></span><span className="color-inputs">
    <input type="color" aria-label={label} value={normalizeHex(value)} onChange={e => { setDraft(e.target.value); onCommit(e.target.value); }} />
    <input type="text" value={draft} maxLength={7} spellCheck={false} onChange={e => setDraft(e.target.value)} onBlur={commit} onKeyDown={e => { if (e.key === 'Enter') { e.preventDefault(); commit(); } }} aria-label={`${label} HEX`} />
  </span></label>;
}
