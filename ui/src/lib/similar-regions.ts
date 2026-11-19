import type { ResultFaces } from './types.ts';
let lastSearch = Date.now() * 1000;
export function nextSimilarSearch() { return ++lastSearch; }

export type SimilarCandidate = {
  id:number; family:string; face_ids:number[]; radius_mm:number|null; semi_angle_rad:number|null;
};
export type SimilarRegions = {candidates:SimilarCandidate[]; truncated:boolean};
export function previewSimilar(document:ResultFaces, regions:SimilarRegions, chosen:number[]) {
  const triangles:number[] = [];
  const faces = new Set<number>();
  for (const candidate of regions.candidates) {
    if (!chosen.includes(candidate.id)) continue;
    for (const id of candidate.face_ids) {
      if (faces.has(id)) continue;
      faces.add(id);
      const face = document.faces[id];
      if (face) {
        for (let i = face.triangle_start; i < face.triangle_start + face.triangle_count; i++) triangles.push(i);
      }
    }
  }
  return triangles;
}
