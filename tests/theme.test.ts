import test from 'node:test';
import assert from 'node:assert/strict';
import { THEME_PRESETS, contrastRatio, deriveTheme, matchingPreset, applyPreset } from '../src/utils/theme.ts';

test('eight distinct presets include four dark and four light palettes with readable text', () => {
  assert.equal(THEME_PRESETS.length, 8);
  assert.equal(THEME_PRESETS.filter(p => p.mode === 'dark').length, 4);
  assert.equal(THEME_PRESETS.filter(p => p.mode === 'light').length, 4);
  assert.equal(new Set(THEME_PRESETS.map(p => p.id)).size, 8);
  assert.equal(new Set(THEME_PRESETS.map(p => JSON.stringify(p.colors))).size, 8);
  for (const preset of THEME_PRESETS) {
    for (const color of Object.values(preset.colors)) assert.match(color, /^#[0-9A-F]{6}$/);
    for (const background of [preset.colors.background, preset.colors.fieldBackground]) {
      assert.ok(contrastRatio(preset.colors.foreground, background) >= 4.5, `${preset.id} body`);
      assert.ok(contrastRatio(preset.colors.mutedForeground, background) >= 4.5, `${preset.id} muted`);
    }
    const derived = deriveTheme(preset.colors);
    assert.ok(contrastRatio(derived.buttonText, derived.button) >= 4.5, `${preset.id} button`);
    assert.equal(derived.colorScheme, preset.mode);
  }
});

test('derived colors preserve five editable roles and choose readable button text', () => {
  const colors = { ...THEME_PRESETS[0].colors, background: '#000000', foreground: '#FFFFFF', accent: '#777777' };
  const derived = deriveTheme(colors);
  assert.equal(derived.border, '#383838');
  assert.equal(derived.button, '#777777');
  assert.equal(derived.buttonText, '#000000');
  assert.equal(deriveTheme({ ...colors, accent: '#000000' }).buttonText, '#FFFFFF');
  assert.equal(deriveTheme({ ...colors, accent: '#FFFFFF' }).buttonText, '#000000');
});

test('preset application preserves opacity and detects customized colors', () => {
  const current = { ...THEME_PRESETS[0].colors, backgroundOpacity: 0.37, contentOpacity: 0.82 };
  const next = applyPreset(current, THEME_PRESETS[6]);
  assert.equal(next.backgroundOpacity, 0.37);
  assert.equal(next.contentOpacity, 0.82);
  assert.deepEqual(current, { ...THEME_PRESETS[0].colors, backgroundOpacity: 0.37, contentOpacity: 0.82 });
  assert.equal(matchingPreset(next)?.id, THEME_PRESETS[6].id);
  assert.equal(matchingPreset({ ...next, accent: '#123456' }), undefined);
  assert.equal(matchingPreset({ ...next, accent: next.accent.toLowerCase() })?.id, THEME_PRESETS[6].id);
});
