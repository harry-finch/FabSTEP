import { test } from 'node:test';
import { strict as assert } from 'node:assert';
import { prepareStepDisplay } from '../src/lib/step-display.ts';
import type { ResultFaces } from '../src/lib/types.ts';

function document():ResultFaces {
  return {request_id:1,source_id:1,deflection_mm:0.01,source_correspondence:{status:'unavailable',reason:'step.provenance_unavailable'},mesh:{vertices:[[0,0,0],[1,0,0],[1,1,0],[0,1,0],[1,0,0],[2,0,0],[2,1,0],[1,1,0]],triangles:[[0,1,2],[0,2,3],[4,5,6],[4,6,7]]},faces:[{id:0,solid_id:0,family:'plane',triangle_start:0,triangle_count:2,adjacent_face_ids:[1]},{id:1,solid_id:0,family:'plane',triangle_start:2,triangle_count:2,adjacent_face_ids:[0]}]};
}
test('STEP contours discard internal diagonals but retain boundaries of adjacent coplanar faces', () => {
  const source = document(), before = JSON.stringify(source);
  const display = prepareStepDisplay(source);
  assert.equal(display.boundaries.length,8*6);
  for (let i=0;i<display.boundaries.length;i+=6) {
    const a=display.boundaries.slice(i,i+3), b=display.boundaries.slice(i+3,i+6);
    assert.ok(a[0] === b[0] || a[1] === b[1], 'no triangulation diagonal');
  }
  assert.notEqual(display.faceSlots[0],display.faceSlots[1]);
  assert.equal(JSON.stringify(source),before,'drawing never mutates native mesh or identities');
  assert.deepEqual(prepareStepDisplay(source),display,'stable within a result');
});
test('closed tessellation of one face has no artificial triangle contours', () => {
  const source = document();
  source.mesh={vertices:[[0,0,0],[1,0,0],[0,1,0],[0,0,1]],triangles:[[0,2,1],[0,1,3],[1,2,3],[2,0,3]]};
  source.faces=[{...source.faces[0],triangle_count:4,adjacent_face_ids:[]}];
  assert.equal(prepareStepDisplay(source).boundaries.length,0);

});
