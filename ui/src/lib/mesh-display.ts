import type { Mesh, RoundedBoxFace } from './types';

/** Preserve native triangle order for the position buffer and raycast indices. */
export function trianglePositions({ vertices, triangles }: Mesh): Float32Array {
  const positions = new Float32Array(triangles.length * 9);
  let offset = 0;
  for (const triangle of triangles) {
    for (const index of triangle) {
      const vertex = vertices[index];
      positions[offset++] = vertex[0];
      positions[offset++] = vertex[1];
      positions[offset++] = vertex[2];
    }
  }
  return positions;
}

function roundedPatchId(owner: RoundedBoxFace): number {
  // The rounded box has 6 planes, 12 cylinders and 8 spherical corners.
  if (owner.kind === 'plane') return owner.axis * 2 + Number(owner.positive);
  if (owner.kind === 'cylinder') return 6 + owner.axis * 4 + Number(owner.positive[0]) * 2 + Number(owner.positive[1]);
  return 18 + Number(owner.positive[0]) * 4 + Number(owner.positive[1]) * 2 + Number(owner.positive[2]);
}

/** Append source edges shared by distinct proven surface patches. */
export function appendPatchBoundaries({ vertices, triangles }: Mesh, owners: readonly (number | RoundedBoxFace)[], points: number[]) {
  const edges = new Map<number, number>();
  // Source vertex limits keep each packed index pair within a safe integer.
  const stride = vertices.length;
  for (let index = 0; index < triangles.length; index++) {
    const triangle = triangles[index];
    const patch = owners[index];
    const owner = typeof patch === 'number' ? patch : patch ? roundedPatchId(patch) : -1;
    for (let corner = 0; corner < 3; corner++) {
      const a = triangle[corner], b = triangle[(corner + 1) % 3];
      const key = Math.min(a, b) * stride + Math.max(a, b);
      const previous = edges.get(key);
      if (previous === undefined) {
        edges.set(key, owner);
      } else {
        if (previous !== owner) points.push(...vertices[a], ...vertices[b]);
        edges.delete(key);
      }
    }
  }
}
