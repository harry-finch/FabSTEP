import { test } from 'node:test';
import assert from 'node:assert/strict';
import { faceCounts, measuredCeilingMm } from '../src/lib/result-summary.ts';
import type { BodyResult } from '../src/lib/types.ts';

const cad: BodyResult['cad'] = { faces:4, solids:1, spherical_faces:1, planar_faces:3, cylindrical_faces:0, conical_faces:0, toroidal_faces:0, volume_mm3:1, bounds_mm:[], kernel:'OCCT' };
const measured = {fidelity:{source_samples:10, result_samples:20, source_to_result_mm:0.004, result_to_source_mm:0.01301}} as BodyResult;

test('a reimported octant is summarized by face family, including uncategorized surfaces', () => {
  assert.deepEqual(faceCounts(cad), [{family:'sphere',count:1}, {family:'plane',count:3}]);
  assert.deepEqual(faceCounts({...cad,faces:5}).at(-1), {family:'other',count:1});
});

test('the displayed maximum rounds up and requires both directions for every body', () => {
  assert.equal(measuredCeilingMm([measured]), 0.014);
  assert.equal(measuredCeilingMm([{...measured, fidelity:{...measured.fidelity!,source_to_result_mm:1e-9, result_to_source_mm:0}}]), 0.001);
  assert.equal(measuredCeilingMm([measured, {...measured, fidelity:null}]), null);
  assert.equal(measuredCeilingMm([{...measured, fidelity:{...measured.fidelity!,source_samples:0}}]), null);
  assert.equal(measuredCeilingMm([]), null);
});
