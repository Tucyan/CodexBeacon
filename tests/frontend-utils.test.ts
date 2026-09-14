import test from 'node:test';
import assert from 'node:assert/strict';
import { normalizeHex, withAlpha, readableRemaining } from '../src/utils/format.ts';
import { effectiveScale, scaledDip } from '../src/utils/scale.ts';
import { formatCountdown, secondsUntil } from '../src/utils/time.ts';
import { quotaLabel, providerStatusLabel, remainingPercent } from '../src/utils/provider.ts';

test('normalizeHex accepts six digit colors and rejects short or invalid input', () => {
  assert.equal(normalizeHex('#abc'), '#000000');
  assert.equal(normalizeHex('abcdef'), '#ABCDEF');
  assert.equal(normalizeHex('#123456'), '#123456');
  assert.equal(normalizeHex('nope'), '#000000');
});

test('withAlpha clamps opacity and emits rgba', () => {
  assert.equal(withAlpha('#123456', 0.5), 'rgba(18, 52, 86, 0.5)');
  assert.equal(withAlpha('#ffffff', 2), 'rgba(255, 255, 255, 1)');
});

test('scale uses global scale for inherited widgets and widget scale otherwise', () => {
  assert.equal(effectiveScale(1.25, 'inherit', 2), 1.25);
  assert.equal(effectiveScale(1.25, 'custom', 0.8), 1);
  assert.equal(scaledDip(100, 1.5), 150);
});

test('countdown time uses UTC epoch seconds and handles expired targets', () => {
  assert.equal(secondsUntil(110, 100), 10);
  assert.equal(secondsUntil(90, 100), 0);
  assert.equal(formatCountdown(3661), '01:01:01');
  assert.equal(formatCountdown(0), '00:00:00');
});

test('provider labels keep nullable quota values honest', () => {
  assert.equal(quotaLabel(null), 'Unavailable');
  assert.equal(quotaLabel({ used: 25, limit: 100 }), '25 / 100');
  assert.equal(providerStatusLabel('offline'), 'Offline');
  assert.equal(providerStatusLabel('ready'), 'Ready');
  assert.equal(remainingPercent({ usedPercent: 72 }), 28);
  assert.equal(remainingPercent({ usedPercent: 55 }), 45);
  assert.equal(remainingPercent({ usedPercent: 120 }), 0);
  assert.equal(remainingPercent({ usedPercent: -10 }), 100);
  assert.equal(remainingPercent({}), null);
});

test('readableRemaining describes countdown values', () => {
  assert.equal(readableRemaining(61), '1m 1s');
  assert.equal(readableRemaining(0), 'Expired');
});
