import test from 'node:test';
import assert from 'node:assert/strict';
import { announcementLabel, elapsedLabel, activeForecast } from '../src/utils/reset.ts';

test('banked announcement is distinct from an immediate quota reset', () => {
  assert.equal(announcementLabel('banked'), '赠送重置次数');
  assert.equal(announcementLabel('regular'), '额度重置公告');
  assert.equal(announcementLabel('unknown'), '重置相关公告');
});
test('relative announcement time handles calendar boundaries and missing dates', () => {
  const now = Date.parse('2026-09-05T01:00:00+08:00');
  assert.equal(elapsedLabel('2026-09-04T23:00:00+08:00', now), '2 小时前');
  assert.equal(elapsedLabel('invalid', now), '时间不可用');
  assert.equal(elapsedLabel('2026-09-06T01:00:00+08:00', now), '时间待核实');
});
test('forecast is separate, nullable, and no longer shown after expiry', () => {
  const watch = { level: 'strong', chancePercent: null, forecastWindow: 'Soon', observedAt: '2026-09-04T00:00:00Z', expiresAt: '2026-09-05T00:00:00Z' };
  assert.equal(activeForecast(null, Date.parse('2026-09-04T12:00:00Z')), null);
  assert.equal(activeForecast(watch, Date.parse('2026-09-05T00:00:00Z')), null);
  assert.equal(activeForecast(watch, Date.parse('2026-09-04T12:00:00Z'))?.chancePercent, null);
});
