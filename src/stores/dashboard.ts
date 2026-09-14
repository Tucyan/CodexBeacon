import { useSyncExternalStore } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { Action, Snapshot, WidgetContent } from '../types/index';
import { initializeRuntime } from '../runtime/tauri';
import { DispatchQueue } from '../services/dispatch';
import { SaveCoordinator } from '../services/save';

type Listener = () => void;

export class DashboardStore {
  private snapshot: Snapshot | null = null;
  private listeners = new Set<Listener>();
  private queue = new DispatchQueue();
  private readonly saver: SaveCoordinator<WidgetContent>;
  private disposed = false;
  private runtimeCleanup: (() => void) | undefined;
  private startPromise: Promise<void> | undefined;
  error: string | null = null;

  constructor() {
    this.saver = new SaveCoordinator<WidgetContent>(
      (id, content) => this.queue.updateContent(id, content).then((snapshot) => { this.receive(snapshot); }),
      (error) => { this.error = error instanceof Error ? error.message : 'Content save failed.'; this.emit(); },
    );
  }

  async start(): Promise<void> {
    this.disposed = false;
    if (this.runtimeCleanup) return;
    if (this.startPromise) return this.startPromise;
    this.startPromise = (async () => {
      try {
        const cleanup = await initializeRuntime((snapshot) => this.receive(snapshot), async (revision) => {
        try {
          await this.saver.flush();
          await invoke('flush_ready', { revision });
        } catch (error) {
          this.error = error instanceof Error ? error.message : 'Content save failed.';
          this.emit();
          throw error;
        }
        });
        if (this.disposed) cleanup(); else this.runtimeCleanup = cleanup;
      } catch (error) {
        if (!this.disposed) { this.error = 'Dashboard runtime is unavailable.'; this.emit(); }
      } finally { this.startPromise = undefined; }
    })();
    return this.startPromise;
  }

  stop(): void { this.disposed = true; this.runtimeCleanup?.(); this.runtimeCleanup = undefined; }
  subscribe = (listener: Listener): (() => void) => { this.listeners.add(listener); return () => this.listeners.delete(listener); };
  getSnapshot = (): Snapshot | null => this.snapshot;

  receive(snapshot: Snapshot): void {
    if (this.disposed) return;
    if (this.snapshot && snapshot.revision <= this.snapshot.revision) return;
    if (this.snapshot) {
      const content = { ...snapshot.content };
      for (const id of snapshot.widgets.map((item) => item.id)) if (this.saver.isDirty(id) && this.snapshot.content[id]) content[id] = this.snapshot.content[id];
      this.snapshot = { ...snapshot, content };
    } else this.snapshot = snapshot;
    this.emit();
  }

  async send(action: Action): Promise<Snapshot | null> {
    try {
      // Rust waits for a self-hidden renderer to acknowledge its flush. Drain
      // content before enqueueing hide so the two IPC operations cannot wait
      // on each other.
      if (action.type === 'hide') await this.saver.flush();
      const next = await this.queue.enqueue(action);
      this.receive(next);
      return next;
    } catch (error) {
      this.error = error instanceof Error ? error.message : 'Action failed.';
      this.emit();
      return null;
    }
  }

  editContent(id: string, content: WidgetContent): void {
    if (!this.snapshot) return;
    this.snapshot = { ...this.snapshot, content: { ...this.snapshot.content, [id]: content } };
    this.emit();
    this.saver.edit(id, content);
  }

  clearError(): void { this.error = null; this.emit(); }
  reportError(message: string): void { this.error = message; this.emit(); }
  retrySaves(id?: string): void { this.clearError(); this.saver.retry(id); }
  whenActionsIdle(): Promise<void> { return this.queue.whenIdle(); }
  private emit(): void { for (const listener of this.listeners) listener(); }
}

export const dashboardStore = new DashboardStore();
export function useDashboard(): Snapshot | null { return useSyncExternalStore(dashboardStore.subscribe, dashboardStore.getSnapshot, dashboardStore.getSnapshot); }
export function useDashboardError(): string | null { return useSyncExternalStore(dashboardStore.subscribe, () => dashboardStore.error, () => dashboardStore.error); }
