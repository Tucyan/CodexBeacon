import type { ThemeConfig } from '../types/index.ts';

export type ThemeColors = Pick<ThemeConfig, 'background' | 'fieldBackground' | 'foreground' | 'mutedForeground' | 'accent'>;
export interface ThemePreset { id: string; name: string; mode: 'dark' | 'light'; colors: ThemeColors }

export const THEME_COLOR_ROLES: Array<{ key: keyof ThemeColors; label: string; hint: string }> = [
  { key: 'background', label: '卡片背景', hint: '组件与设置窗口的背景' },
  { key: 'fieldBackground', label: '输入区背景', hint: '备忘录、待办与输入控件' },
  { key: 'foreground', label: '主要文字', hint: '标题和正文' },
  { key: 'mutedForeground', label: '次要文字', hint: '占位提示、时间和说明' },
  { key: 'accent', label: '强调色', hint: '按钮、焦点、勾选和进度条' },
];

export const THEME_PRESETS: ThemePreset[] = [
  { id: 'midnight', name: '午夜蓝', mode: 'dark', colors: { background: '#17212B', fieldBackground: '#101821', foreground: '#EEF4F7', mutedForeground: '#ACBECD', accent: '#8EC5FF' } },
  { id: 'forest', name: '深林', mode: 'dark', colors: { background: '#20332E', fieldBackground: '#14251F', foreground: '#F0F5EC', mutedForeground: '#AFBFB2', accent: '#A4D9A7' } },
  { id: 'violet', name: '暮紫', mode: 'dark', colors: { background: '#302638', fieldBackground: '#221B2B', foreground: '#F6EFFB', mutedForeground: '#C3B4CF', accent: '#C9ABF4' } },
  { id: 'ember', name: '暖炭', mode: 'dark', colors: { background: '#342C28', fieldBackground: '#241E1A', foreground: '#FAF2E8', mutedForeground: '#CABBAD', accent: '#F0BD82' } },
  { id: 'paper', name: '米纸', mode: 'light', colors: { background: '#F4F1E8', fieldBackground: '#FFFEFA', foreground: '#343B30', mutedForeground: '#626958', accent: '#4C7050' } },
  { id: 'sea', name: '海盐', mode: 'light', colors: { background: '#E4F1F1', fieldBackground: '#F7FCFC', foreground: '#234543', mutedForeground: '#526E6C', accent: '#267B79' } },
  { id: 'lavender', name: '浅薰衣草', mode: 'light', colors: { background: '#F0EBF8', fieldBackground: '#FCFAFF', foreground: '#413750', mutedForeground: '#71617E', accent: '#7C58A2' } },
  { id: 'rose', name: '晨樱', mode: 'light', colors: { background: '#FAEDEE', fieldBackground: '#FFFAFA', foreground: '#51383E', mutedForeground: '#7E6168', accent: '#A34D68' } },
];

function channels(hex: string): number[] {
  return [1, 3, 5].map(offset => Number.parseInt(hex.slice(offset, offset + 2), 16));
}

function luminance(hex: string): number {
  const [r, g, b] = channels(hex).map(value => {
    const channel = value / 255;
    return channel <= 0.04045 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

export function contrastRatio(a: string, b: string): number {
  const first = luminance(a), second = luminance(b);
  return (Math.max(first, second) + 0.05) / (Math.min(first, second) + 0.05);
}

function mix(a: string, b: string, weight: number): string {
  const second = channels(b);
  return `#${channels(a).map((value, index) => Math.round(value * (1 - weight) + second[index] * weight).toString(16).padStart(2, '0')).join('').toUpperCase()}`;
}

export function deriveTheme(colors: ThemeColors) {
  return {
    border: mix(colors.background, colors.foreground, 0.22),
    button: colors.accent,
    buttonText: contrastRatio(colors.accent, '#000000') >= contrastRatio(colors.accent, '#FFFFFF') ? '#000000' : '#FFFFFF',
    colorScheme: luminance(colors.background) > 0.179 ? 'light' as const : 'dark' as const,
  };
}

export function applyPreset(theme: ThemeConfig, preset: ThemePreset): ThemeConfig {
  return { ...theme, ...preset.colors };
}

export function matchingPreset(theme: ThemeColors): ThemePreset | undefined {
  return THEME_PRESETS.find(preset => THEME_COLOR_ROLES.every(({ key }) => preset.colors[key].toUpperCase() === theme[key].toUpperCase()));
}

export function themeVariables(theme: ThemeConfig) {
  const derived = deriveTheme(theme);
  return {
    '--accent': theme.accent, '--background': theme.background,
    '--field-background': theme.fieldBackground, '--foreground': theme.foreground,
    '--muted-foreground': theme.mutedForeground, '--border': derived.border,
    '--button-background': derived.button, '--button-foreground': derived.buttonText,
    '--background-opacity': theme.backgroundOpacity, '--content-opacity': theme.contentOpacity,
    colorScheme: derived.colorScheme,
  };
}
