import type { Action, Snapshot, WidgetContent } from '../types/index';
import { dispatch } from '../runtime/tauri';

export class DispatchQueue {
  private pending: Promise<Snapshot> = Promise.resolve(undefined as unknown as Snapshot);

  enqueue(action: Action): Promise<Snapshot> {
    const next = this.pending.catch(() => undefined as unknown as Snapshot).then(() => dispatch(action));
    this.pending = next;
    return next;
  }

  updateContent(id: string, content: WidgetContent): Promise<Snapshot> {
    return this.enqueue({ type: 'content', id, content });
  }

  whenIdle(): Promise<void> {
    return this.pending.then(() => undefined, () => undefined);
  }
}
