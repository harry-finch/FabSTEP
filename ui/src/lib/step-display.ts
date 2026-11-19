import type { DeviationReport } from './deviation-map.ts';
import type { ResultFaces } from './types.ts';

export const STEP_PALETTE_SIZE = 8;
export type StepDisplay = { faceSlots:Uint8Array; boundaries:Float32Array; deviation?:DeviationReport };
/** Called only after StepSelectionGraph validation, in the STEP worker. */
export function prepareStepDisplay(document:ResultFaces):StepDisplay {
  const { mesh, faces } = document;
  if (mesh.triangles.length > 100_000 || mesh.vertices.length > 300_000 || faces.length > 100_000) throw new Error('step.invalid_preview');
  const faceSlots = new Uint8Array(faces.length);
  const assigned = new Uint8Array(faces.length);
  const points:number[] = [];
  for (const face of faces) {
    // Different neighbouring faces get different slots whenever the palette permits it.
    // The slot denotes identity in this result, never surface family or quality.
    const counts = new Uint32Array(STEP_PALETTE_SIZE);
    for (const neighbor of face.adjacent_face_ids) if (assigned[neighbor]) counts[faceSlots[neighbor]]++;
    let slot = face.id % STEP_PALETTE_SIZE;
    for (let offset = 1; offset < STEP_PALETTE_SIZE; offset++) {
      const candidate = (face.id + offset) % STEP_PALETTE_SIZE;
      if (counts[candidate] < counts[slot]) slot = candidate;
    }
    faceSlots[face.id] = slot; assigned[face.id] = 1;
    // Native tessellation shares indices within a face, but not between faces.
    // Single-incidence edges trace its mesh boundary; interior diagonals disappear.
    // Periodic seams can remain: these are display polylines, not exact BRep curves.
    const edges = new Map<number,{ a:number; b:number; count:number }>();
    for (let i = face.triangle_start; i < face.triangle_start + face.triangle_count; i++) {
      const triangle = mesh.triangles[i];
      for (let corner = 0; corner < 3; corner++) {
        const a = triangle[corner], b = triangle[(corner + 1) % 3];
        const key = Math.min(a,b) * mesh.vertices.length + Math.max(a,b);
        const previous = edges.get(key);
        if (previous) previous.count++;
        else edges.set(key,{a,b,count:1});
      }
    }
    for (const edge of edges.values()) if (edge.count === 1) points.push(...mesh.vertices[edge.a], ...mesh.vertices[edge.b]);
  }
  return {faceSlots, boundaries:new Float32Array(points)};
}
