import { test } from 'node:test';
import { strict as assert } from 'node:assert';
import { largeStlTriangles } from '../src/lib/large-stl.ts';

test('large STL admission requires exact binary framing and finite source cap', () => {
  for (const count of [100_000, 100_001, 540_170, 600_000, 600_001]) {
    const data = new Uint8Array(84 + 50 * count);
    new DataView(data.buffer).setUint32(80, count, true);
    assert.equal(largeStlTriangles(data, 'stl'), count > 100_000 && count <= 600_000 ? count : null);
    assert.equal(largeStlTriangles(data.subarray(0, data.length - 1), 'stl'), null);
    assert.equal(largeStlTriangles(data, '3mf'), null);
  }
  assert.equal(largeStlTriangles(new Uint8Array(83), 'stl'), null);
});
