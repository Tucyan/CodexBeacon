type SaveFn<T> = (id: string, value: T) => Promise<void>;

type DirtyValue<T> = { version: number; value: T };

/** Serializes content writes while retaining the latest local version per widget. */
export class SaveCoordinator<T> {
  private readonly save: SaveFn<T>;
  private readonly onError: ((error: unknown) => void) | undefined;
  private queue: Promise<void> = Promise.resolve();
  private versions = new Map<string, number>();
  private dirty = new Map<string, DirtyValue<T>>();
  private failures: unknown[] = [];

  constructor(save: SaveFn<T>, onError?: (error: unknown) => void) { this.save = save; this.onError = onError; }

  edit(id: string, value: T): void {
    // A new user edit is an explicit retry boundary for an earlier failure.
    this.failures = [];
    const version = (this.versions.get(id) ?? 0) + 1;
    this.versions.set(id, version);
    this.dirty.set(id, { version, value });
    this.schedule(id, value, version);
  }

  retry(id?: string): void {
    this.failures = [];
    for (const [dirtyId, dirtyValue] of this.dirty) {
      if (id && id !== dirtyId) continue;
      this.schedule(dirtyId, dirtyValue.value, dirtyValue.version);
    }
  }

  private schedule(id: string, value: T, version: number): void {
    const operation = async () => {
      try {
        await this.save(id, value);
        const current = this.dirty.get(id);
        if (current?.version === version) this.dirty.delete(id);
      } catch (error) {
        this.failures.push(error);
        this.onError?.(error);
      }
    };
    this.queue = this.queue.then(operation, operation);
  }

  isDirty(id: string): boolean { return this.dirty.has(id); }
  hasDirty(): boolean { return this.dirty.size > 0; }

  async flush(): Promise<void> {
    while (true) {
      const current = this.queue;
      await current;
      if (current === this.queue) break;
    }
    if (this.failures.length) {
      const error = this.failures[0];
      throw error instanceof Error ? error : new Error('Content save failed.');
    }
    if (this.hasDirty()) throw new Error('Unsaved content remains. Retry saves before continuing.');
  }

  drain(): Promise<void> { return this.flush(); }
}
