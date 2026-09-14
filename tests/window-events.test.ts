import test from 'node:test';
import assert from 'node:assert/strict';
import { initializeRuntime, subscribeToAppearance, subscribeToSnapshots } from '../src/runtime/tauri.ts';

// Exercise the real @tauri-apps/api registration against a small IPC bridge.
// Tauri's event/listener.rs match_any_or_filter explicitly lets Any listeners
// receive even targeted emits. Merely changing Rust to emit_to is insufficient.
test('state, appearance and flush listeners stay in their own window', async () => {
  let label = 'widget-memo'; let nextId = 0;
  const callbacks = new Map<number, (event: unknown) => void>();
  const listeners: Array<{ id: number; event: string; target: { kind: string; label?: string }; handler: number }> = [];
  const received: Record<string, string[]> = { 'widget-memo': [], 'widget-todo': [], settings: [] };
  const globals = globalThis as any;
  const previous = globals.window;
  globals.window = {
    __TAURI_INTERNALS__: {
      metadata: { get currentWindow() { return { label }; } },
      transformCallback: (callback: (event: unknown) => void) => { const id = ++nextId; callbacks.set(id, callback); return id; },
      invoke: async (command: string, args: any) => {
        if (command === 'plugin:event|listen') { const id = ++nextId; listeners.push({ ...args, id }); return id; }
        if (command === 'get_snapshot') return { revision: 0, widgets: [] };
        return undefined;
      },
    },
    __TAURI_EVENT_PLUGIN_INTERNALS__: { unregisterListener() {} },
  };
  try {
    for (const current of Object.keys(received)) {
      label = current;
      await subscribeToSnapshots((value) => received[current].push(`state:${value.widgets[0]?.id}`));
      await subscribeToAppearance(() => received[current].push('appearance'), () => received[current].push('prepare'));
      await initializeRuntime(() => {}, async () => { received[current].push('flush'); });
    }
    function emitTo(target: string, event: string, payload: unknown) {
      for (const item of listeners) {
        if (item.event === event && (item.target.kind === 'Any' || item.target.label === target)) {
          callbacks.get(item.handler)?.({ event, id: item.id, payload });
        }
      }
    }
    emitTo('widget-memo', 'dashboard-state', { revision: 1, widgets: [{ id: 'memo' }] });
    emitTo('widget-todo', 'dashboard-state', { revision: 1, widgets: [{ id: 'todo' }] });
    emitTo('widget-memo', 'dashboard-appearance-prepare', { revision: 1, animate: true });
    emitTo('widget-memo', 'dashboard-appearance', { revision: 1, animate: true });
    emitTo('widget-todo', 'dashboard-flush', { revision: 1 });
    assert.deepEqual(received, {
      'widget-memo': ['state:memo', 'prepare', 'appearance'],
      'widget-todo': ['state:todo', 'flush'],
      settings: [],
    });
  } finally { globals.window = previous; }
});
