import { useEffect, useState } from 'react';
import type { AppSettings, BackupOperation, BackupStatus } from '../types/index';
import { backupNow, exportBackup, getBackupStatus, importBackup } from '../runtime/tauri';
import { dashboardStore } from '../stores/dashboard';
import './backup.css';

type Operation = 'backup' | 'export' | 'import' | null;

function safeErrorMessage(error?: unknown): string {
  const code = String(error ?? '');
  if (code.includes('backup_invalid')) return '这个文件不是受支持的 Desktop Dashboard 备份。';
  if (code.includes('backup_too_large')) return '备份文件超过 32 MiB，无法导入。';
  if (code.includes('backup_read_failed')) return '无法读取所选备份文件。';
  if (code.includes('backup_write_failed')) return '无法写入所选位置，请检查路径和磁盘空间。';
  if (code.includes('database_save_failed')) return '导入未保存，现有设置保持不变。';
  if (code.includes('renderer_flush_failed')) return '组件内容尚未保存，请稍后重试。';
  return '备份操作失败，请稍后重试。';
}

function formatTimestamp(epochSeconds: number | null): string {
  return epochSeconds === null ? '尚未备份' : new Date(epochSeconds * 1000).toLocaleString();
}

function operationMessage(operation: BackupOperation): string {
  if (operation.status === 'cancelled') return '已取消。';
  if (operation.status === 'imported') return '备份已导入，设置和内容已恢复。';
  return operation.path ? `备份已保存：${operation.path}` : '备份已保存。';
}

export function BackupSettings({ settings, save }: { settings: AppSettings; save: (settings: AppSettings) => void }) {
  const [intervalDraft, setIntervalDraft] = useState(String(settings.backupIntervalDays));
  const [status, setStatus] = useState<BackupStatus | null>(null);
  const [statusLoading, setStatusLoading] = useState(true);
  const [operation, setOperation] = useState<Operation>(null);
  const [feedback, setFeedback] = useState<string | null>(null);

  useEffect(() => setIntervalDraft(String(settings.backupIntervalDays)), [settings.backupIntervalDays]);
  useEffect(() => {
    let active = true;
    void getBackupStatus().then((next) => { if (active) setStatus(next); }).catch((error) => { if (active) setFeedback(safeErrorMessage(error)); }).finally(() => { if (active) setStatusLoading(false); });
    return () => { active = false; };
  }, []);

  const refreshStatus = async () => {
    try { setStatus(await getBackupStatus()); } catch (error) { setFeedback(safeErrorMessage(error)); }
  };
  const busy = statusLoading || operation !== null;
  const commitInterval = () => {
    const parsed = Number(intervalDraft.trim());
    if (!Number.isFinite(parsed)) { setIntervalDraft(String(settings.backupIntervalDays)); return; }
    const next = Math.max(1, Math.min(365, Math.round(parsed)));
    setIntervalDraft(String(next));
    if (next !== settings.backupIntervalDays) save({ ...settings, backupIntervalDays: next });
  };
  const run = async (kind: Exclude<Operation, null>, action: () => Promise<BackupOperation>) => {
    setOperation(kind); setFeedback(null);
    try {
      await dashboardStore.whenActionsIdle();
      const result = await action();
      setFeedback(operationMessage(result));
      if (kind === 'backup' || kind === 'import') await refreshStatus();
    } catch (error) { setFeedback(safeErrorMessage(error)); }
    finally { setOperation(null); }
  };
  const beginImport = () => {
    if (!window.confirm('导入备份将替换现有设置、布局以及 Memo、Todo、Countdown 内容。是否继续？')) return;
    void run('import', importBackup);
  };

  return <div className="settings-card backup-settings">
    <h2>备份</h2>
    <p className="backup-explanation">备份包含主题和行为设置、组件状态与布局，以及 Memo、Todo、Countdown 内容。</p>
    <p className="muted">不包含 Codex/Provider 状态、账户响应、访问凭据或原始网络响应。</p>
    <label className="backup-interval">自动备份间隔（天）<input type="text" inputMode="numeric" minLength={1} maxLength={3} value={intervalDraft} aria-label="自动备份间隔天数" onChange={(event) => setIntervalDraft(event.target.value)} onBlur={commitInterval} onKeyDown={(event) => { if (event.key === 'Enter') { event.preventDefault(); commitInterval(); } }} /></label>
    <div className="backup-actions"><button disabled={busy} onClick={() => void run('backup', backupNow)}>{operation === 'backup' ? '备份中…' : '立即备份'}</button><button disabled={busy} onClick={() => void run('export', exportBackup)}>{operation === 'export' ? '导出中…' : '导出 JSON'}</button><button disabled={busy} onClick={beginImport}>{operation === 'import' ? '导入中…' : '导入 JSON'}</button></div>
    <div className="backup-status" aria-live="polite">{statusLoading ? '正在读取备份状态…' : feedback ?? '就绪。'}</div>
    <dl className="backup-meta"><div><dt>最近自动/立即备份</dt><dd>{formatTimestamp(status?.lastBackupAt ?? null)}</dd></div>{status?.lastBackupPath && <div><dt>路径</dt><dd className="backup-path">{status.lastBackupPath}</dd></div>}</dl>
  </div>;
}
