import { Box3, Vector3, type Material } from 'three';

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

/** Interpolate signed distances, so a face on a box limit stays exactly zero
 * across the triangle even with a perspective camera. */
export function bindClipBox(materials: Material[]): (box: Box3 | null) => void {
  let enabled = false;
  const lower = { value: new Vector3() };
  const upper = { value: new Vector3() };
  for (const material of materials) {
    material.customProgramCacheKey = () => enabled ? 'fabstep-clip-box' : '';
    material.onBeforeCompile = shader => {
      if (!enabled) return;
      shader.uniforms.boxLower = lower;
      shader.uniforms.boxUpper = upper;
      // Centroid sampling keeps antialiased silhouette pixels inside the primitive.
      const varyings = 'centroid varying vec3 fromLower; centroid varying vec3 fromUpper;';
      shader.vertexShader = shader.vertexShader
        .replace('#include <clipping_planes_pars_vertex>', `${varyings}
          uniform vec3 boxLower;
          uniform vec3 boxUpper;`)
        .replace('#include <clipping_planes_vertex>', `
          vec3 boxPoint = (modelMatrix * vec4(transformed, 1.0)).xyz;
          fromLower = boxPoint - boxLower;
          fromUpper = boxUpper - boxPoint;`);
      shader.fragmentShader = shader.fragmentShader
        .replace('#include <clipping_planes_pars_fragment>', varyings)
        .replace('#include <clipping_planes_fragment>', `
          if (any(lessThan(min(fromLower, fromUpper), vec3(0.0)))) discard;`);
    };
  }
  return box => {
    if (box) {
      lower.value.copy(box.min);
      upper.value.copy(box.max);
    }
    if (enabled !== (box !== null)) {
      enabled = box !== null;
      for (const material of materials) material.needsUpdate = true;
    }
  };
}
