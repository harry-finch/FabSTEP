import { test } from 'node:test';
import assert from 'node:assert/strict';
import { Box3, DoubleSide, Mesh, MeshBasicMaterial, PerspectiveCamera, PlaneGeometry, Raycaster, Vector2, Vector3 } from 'three';
import { clipExtent, clipPlanes, moveClipLimit, axes } from '../src/lib/clip-box.ts';
import { pickVisibleSourceTriangle, pickVisibleStepFace } from '../src/lib/source-picking.ts';

test('all six limits clip on the same side as CPU selection', () => {
  const box = new Box3(new Vector3(-8, 2, 3), new Vector3(4, 10, 7));
  const middle = box.getCenter(new Vector3());
  const planes = clipPlanes(box);
  for (const axis of axes) for (const value of [box.min[axis] - 0.01, box.min[axis], middle[axis], box.max[axis], box.max[axis] + 0.01]) {
    const p = middle.clone();
    p[axis] = value;
    assert.equal(planes.every(plane => plane.distanceToPoint(p) >= 0), box.containsPoint(p));
  }
});

test('bounds respect source extents and keep opposite faces strictly separated', () => {
  const extent = clipExtent(new Box3(new Vector3(-2, 1e8, 0), new Vector3(4, 1e8 + 1, 0)))!;
  let box = extent.clone();
  assert.ok(extent.min.z < 0 && extent.max.z > 0);
  for (const axis of axes) {
    box = moveClipLimit(box, extent, axis, 'min', 1e12);
    box = moveClipLimit(box, extent, axis, 'max', -1e12);
    assert.ok(box.min[axis] < box.max[axis]);
    assert.ok(extent.containsBox(box));
    box = moveClipLimit(box, extent, axis, 'min', -1e12);
    box = moveClipLimit(box, extent, axis, 'max', 1e12);
  }
  assert.deepEqual(box, extent);
  assert.equal(moveClipLimit(box, extent, 'x', 'max', NaN), box);
  assert.equal(clipExtent(new Box3()), null);
});

test('selection and paint reach the first visible hit inside the box', () => {
  const camera = new PerspectiveCamera(45, 1, 0.01, 100);
  camera.position.set(0, 0, 10);
  camera.lookAt(0, 0, 0);
  const material = new MeshBasicMaterial({ side: DoubleSide }), geometry = new PlaneGeometry(4, 4);
  const meshes = [2, 0, -2].map((z, i) => {
    const m = new Mesh(geometry, material);
    m.position.z = z;
    m.userData = { stepFaces: [i, i], previewBodyId: String(i), sourceTriangles: [5, 5] };
    return m;
  });
  const box = new Box3(new Vector3(0, -2, -1), new Vector3(2, 2, 1));
  const ray = new Raycaster();
  // Each wall spans the clipping boundary: visibility follows the point of impact.
  for (const x of [-0.1, 0.1]) {
    const pointer = new Vector2(x, 0.1);
    assert.equal(pickVisibleStepFace(ray, meshes, pointer, camera), 0);
    assert.equal(pickVisibleStepFace(ray, meshes, pointer, camera, box), x > 0 ? 1 : null);
    assert.equal(pickVisibleSourceTriangle(ray, meshes, pointer, camera, box)?.bodyId, x > 0 ? '1' : undefined);
  }
  material.dispose();
  geometry.dispose();
});
