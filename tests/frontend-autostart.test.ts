import assert from 'node:assert/strict';
import test from 'node:test';
import { autostartErrorMessage } from '../src/settings/autostart.ts';

test('autostart errors use a stable actionable message without exposing backend details', () => {
  assert.equal(
    autostartErrorMessage('autostart_write_failed: C:\\private\\Desktop-Dashboard.exe'),
    '无法更新 Windows 开机启动设置，请重试。',
  );
  assert.equal(
    autostartErrorMessage(new Error('registry access denied')),
    '无法更新 Windows 开机启动设置，请重试。',
  );
});
