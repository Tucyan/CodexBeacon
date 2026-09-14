import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import type { Action, AutostartStatus, BackupOperation, BackupStatus, Snapshot } from '../types/index';

export async function getSnapshot(): Promise<Snapshot> {
  return invoke<Snapshot>('get_snapshot');
}

export async function widgetReady(): Promise<void> {
  await invoke('widget_ready');
}

export async function dispatch(action: Action): Promise<Snapshot> {
  return invoke<Snapshot>('dispatch', { action });
}

export async function getBackupStatus(): Promise<BackupStatus> {
  return invoke<BackupStatus>('get_backup_status');
}

export async function backupNow(): Promise<BackupOperation> {
  return invoke<BackupOperation>('backup_now');
}

export async function exportBackup(): Promise<BackupOperation> {
  return invoke<BackupOperation>('export_backup');
}

export async function importBackup(): Promise<BackupOperation> {
  return invoke<BackupOperation>('import_backup');
}

export async function getAutostartStatus(): Promise<AutostartStatus> {
  return invoke<AutostartStatus>('get_autostart_status');
}

export async function setAutostartEnabled(enabled: boolean): Promise<AutostartStatus> {
  return invoke<AutostartStatus>('set_autostart_enabled', { enabled });
}

export async function startDrag(): Promise<void> {
  await invoke('start_drag');
}

export async function startResize(direction: string): Promise<void> {
  await invoke('start_resize', { direction });
}

export async function subscribeToSnapshots(onSnapshot: (snapshot: Snapshot) => void): Promise<UnlistenFn> {
  return listen<Snapshot>('dashboard-state', (event) => onSnapshot(event.payload), { target: getCurrentWindow().label });
}

export async function subscribeToAppearance(onAppearance: (event: { revision: number; animate: boolean }) => void, onPrepare?: (event: { revision: number; animate: boolean }) => void): Promise<UnlistenFn> {
  let latestToken = -1;
  const handle = (event: { payload: unknown }) => {
    const payload = event.payload as Partial<{ revision: number; animate: boolean }>;
    if (typeof payload.revision === 'number' && payload.revision >= latestToken) {
      latestToken = payload.revision;
      onAppearance({ revision: payload.revision, animate: payload.animate !== false });
    }
  };
  const handlePrepare = (event: { payload: unknown }) => {
    const payload = event.payload as Partial<{ revision: number; animate: boolean }>;
    if (typeof payload.revision !== 'number' || payload.revision < latestToken) return;
    latestToken = payload.revision;
    onPrepare?.({ revision: payload.revision, animate: payload.animate !== false });
    void invoke('appearance_ready', { revision: payload.revision }).catch(() => undefined);
  };
  const options = { target: getCurrentWindow().label };
  const cleanups = await Promise.all([listen('dashboard-appearance', handle, options), listen('dashboard-appearance-prepare', handlePrepare, options)]);
  return () => { for (const cleanup of cleanups) cleanup(); };
}

export async function initializeRuntime(onSnapshot: (snapshot: Snapshot) => void, onFlush?: (revision: number) => Promise<void>): Promise<() => void> {
  // Subscribe before taking the snapshot so a broadcast between the two is not lost.
  const unlisten = await subscribeToSnapshots(onSnapshot);
  const unlistenFlush = onFlush ? await listen<{ revision: number }>('dashboard-flush', (event) => { void onFlush(event.payload.revision).catch(() => undefined); }, { target: getCurrentWindow().label }) : undefined;
  try {
    onSnapshot(await getSnapshot());
  } catch (error) {
    unlisten();
    unlistenFlush?.();
    throw error;
  }
  return () => { unlisten(); unlistenFlush?.(); };
}
