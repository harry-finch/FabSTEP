import { test } from 'node:test';
import { strict as assert } from 'node:assert';
import { uploadSource, type SourceBridge } from '../src/lib/source.ts';

test('raw transfer retains the exact subview and carries only bounded metadata', async () => {
  const backing = new Uint8Array([99, 1, 2, 3, 88]);
  const view = backing.subarray(1, 4);
  let calls = 0;
  const bridge: SourceBridge = { invoke: async <T>(command: string, data: Record<string, unknown> | Uint8Array, options?: { headers: Record<string, string> }) => {
    calls++; assert.equal(command, 'upload_source'); assert.equal(data, view);
    assert.deepEqual([...data as Uint8Array], [1, 2, 3]);
    assert.deepEqual(options?.headers, { 'x-fabstep-source-id':'42', 'x-fabstep-format':'3mf' });
    return undefined as T;
  } };
  await uploadSource(bridge, view, 42, '3mf');
  assert.equal(calls, 1);
});

test('over-limit raw source is rejected before IPC', async () => {
  let called = false;
  const bridge: SourceBridge = { invoke: async <T>() => { called = true; return undefined as T; } };
  await assert.rejects(uploadSource(bridge, new Uint8Array(64 * 1024 * 1024 + 1), 7, 'stl'), (e: unknown) => (e as { code:string }).code === 'input.too_large');
  assert.equal(called, false);
});
