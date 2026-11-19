import { Box3, Plane, Vector3 } from 'three';

export const axes = ['x', 'y', 'z'] as const;
export type Axis = typeof axes[number];
export type Limit = 'min' | 'max';

export function clipExtent(box: Box3): Box3 | null {
  if (box.isEmpty() || ![...box.min.toArray(), ...box.max.toArray()].every(Number.isFinite)) return null;
  const extent = box.clone();
  for (const axis of axes) {
    if (extent.min[axis] === extent.max[axis]) {
      const margin = Math.max(1e-6, Math.abs(extent.min[axis]) * 1e-12);
      extent.min[axis] -= margin;
      extent.max[axis] += margin;
    }
  }
  return extent;
}

/** Stop before the opposite face, even for small parts far from the origin. */
export function moveClipLimit(box: Box3, extent: Box3, axis: Axis, side: Limit, value: number): Box3 {
  if (!Number.isFinite(value)) return box;
  const span = extent.max[axis] - extent.min[axis];
  const gap = Math.min(span, Math.max(span * 1e-6, Number.EPSILON * Math.max(Math.abs(extent.min[axis]), Math.abs(extent.max[axis])) * 4));
  const low = side === 'min' ? extent.min[axis] : box.min[axis] + gap;
  const high = side === 'max' ? extent.max[axis] : box.max[axis] - gap;
  const next = box.clone();
  next[side][axis] = Math.min(high, Math.max(low, value));
  return next;
}

export function clipPlanes(box: Box3): Plane[] {
  return axes.flatMap(axis => {
    const normal = new Vector3();
    normal[axis] = 1;
    return [new Plane(normal, -box.min[axis]), new Plane(normal.clone().negate(), box.max[axis])];
  });
}
