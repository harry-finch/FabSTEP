import type { ConversionResult } from './types.ts';

const families = [
  ['sphere', 'spherical_faces'], ['cylinder', 'cylindrical_faces'],
  ['cone', 'conical_faces'], ['torus', 'toroidal_faces'],
  ['revolution', 'revolution_faces'], ['plane', 'planar_faces'],
] as const;
type Family = typeof families[number][0] | 'other';

export function faceCounts(cad: ConversionResult['cad']) {
  const counts: {family:Family; count:number}[] = [];
  let classified = 0;
  for (const [family, key] of families) {
    const count = cad[key] ?? 0;
    if (count > 0) counts.push({family, count});
    classified += count;
  }
  if (classified < cad.faces) counts.push({family:'other', count:cad.faces - classified});
  return counts;
}

/** Both sample directions must cover every body to label a document-wide maximum. */
export function measuredCeilingMm(bodies: ConversionResult['bodies']): number | null {
  if (!bodies.length) return null;
  let maximum = 0;
  for (const { fidelity } of bodies) {
    if (!fidelity || !(fidelity.source_samples > 0 && fidelity.result_samples > 0)) return null;
    for (const value of [fidelity.source_to_result_mm, fidelity.result_to_source_mm]) {
      if (!Number.isFinite(value) || value < 0) return null;
      maximum = Math.max(maximum, value);
    }
  }
  const thousandths = Math.ceil(maximum * 1000);
  const rounded = thousandths / 1000;
  return rounded >= maximum ? rounded : (thousandths + 1) / 1000;
}
