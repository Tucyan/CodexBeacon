import test from 'node:test';
import assert from 'node:assert/strict';
import { SaveCoordinator } from '../src/services/save.ts';

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((res, rej) => { resolve = res; reject = rej; });
  return { promise, resolve, reject };
}

test('saves edits serially and keeps a newer edit dirty after an older ack', async () => {
  const first = deferred<void>();
  const second = deferred<void>();
  const calls: string[] = [];
  const coordinator = new SaveCoordinator<string>(async (id, value) => {
    calls.push(`${id}:${value}`);
    if (value === 'first') await first.promise;
    else await second.promise;
  });
  coordinator.edit('memo', 'first');
  coordinator.edit('memo', 'second');
  await Promise.resolve();
  assert.deepEqual(calls, ['memo:first']);
  first.resolve();
  await new Promise((resolve) => setTimeout(resolve, 0));
  assert.equal(coordinator.isDirty('memo'), true);
  assert.deepEqual(calls, ['memo:first', 'memo:second']);
  second.resolve();
  await coordinator.flush();
  assert.equal(coordinator.isDirty('memo'), false);
});

test('flush rejects a failed save and leaves its content dirty', async () => {
  const coordinator = new SaveCoordinator<string>(async () => { throw new Error('disk unavailable'); });
  coordinator.edit('memo', 'keep me');
  await assert.rejects(coordinator.flush(), /disk unavailable/);
  assert.equal(coordinator.isDirty('memo'), true);
});

test('flush continues rejecting until the dirty value is successfully retried', async () => {
  let attempts = 0;
  const coordinator = new SaveCoordinator<string>(async () => {
    attempts += 1;
    if (attempts === 1) throw new Error('temporary failure');
  });
  coordinator.edit('memo', 'retry me');
  await assert.rejects(coordinator.flush(), /temporary failure/);
  await assert.rejects(coordinator.flush(), /temporary failure/);
  coordinator.edit('memo', 'retry me');
  await coordinator.flush();
  assert.equal(coordinator.isDirty('memo'), false);
});

test('a failure on one widget keeps flush failed after another widget succeeds', async () => {
  const coordinator = new SaveCoordinator<string>(async (id) => {
    if (id === 'memo') throw new Error('memo failed');
  });
  coordinator.edit('memo', 'bad');
  coordinator.edit('todo', 'good');
  await assert.rejects(coordinator.flush(), /memo failed/);
  assert.equal(coordinator.isDirty('memo'), true);
  assert.equal(coordinator.isDirty('todo'), false);
  await assert.rejects(coordinator.flush(), /memo failed/);
  coordinator.retry('memo');
  await assert.rejects(coordinator.flush(), /memo failed/);
});

test('retry requeues failed dirty values after the save function recovers', async () => {
  let available = false;
  const coordinator = new SaveCoordinator<string>(async () => { if (!available) throw new Error('offline'); });
  coordinator.edit('memo', 'keep');
  await assert.rejects(coordinator.flush(), /offline/);
  available = true;
  coordinator.retry('memo');
  await coordinator.flush();
  assert.equal(coordinator.isDirty('memo'), false);
});
