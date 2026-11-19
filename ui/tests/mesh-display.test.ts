import { strict as assert } from 'node:assert';
import { test } from 'node:test';
import { appendPatchBoundaries, trianglePositions } from '../src/lib/mesh-display.ts';
import type { RoundedBoxFace } from '../src/lib/types.ts';

const mesh = {
  vertices: [[0.1, 0, 0], [2, 0, 0], [0, 1, 0], [2, 1, 1]],
  triangles: [[0, 1, 2], [2, 1, 3]],
};

test('display positions keep the order and winding used by face picking', () => {
  assert.deepEqual(trianglePositions(mesh), new Float32Array([
    ...mesh.vertices[0], ...mesh.vertices[1], ...mesh.vertices[2],
    ...mesh.vertices[2], ...mesh.vertices[1], ...mesh.vertices[3],
  ]));
  assert.deepEqual(trianglePositions({ vertices: [], triangles: [] }), new Float32Array());
});

test('only a shared edge between different patch owners is drawn', () => {
  const plane: RoundedBoxFace = { kind: 'plane', axis: 2, positive: false };
  const samePlane: RoundedBoxFace = { kind: 'plane', axis: 2, positive: false };
  const cylinder: RoundedBoxFace = { kind: 'cylinder', axis: 0, positive: [false, true] };
  const points: number[] = [];
  appendPatchBoundaries(mesh, [plane, samePlane], points);
  appendPatchBoundaries(mesh, [3, 3], points);
  assert.deepEqual(points, []);
  appendPatchBoundaries(mesh, [plane, cylinder], points);
  appendPatchBoundaries(mesh, [3, 4], points);
  const shared = [...mesh.vertices[2], ...mesh.vertices[1]];
  assert.deepEqual(points, [...shared, ...shared]);
});
