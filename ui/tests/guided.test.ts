import { test } from 'node:test';
import { strict as assert } from 'node:assert';
import { canPrepareCorrection, guidedFaces, guidedFamilyLabel, acceptsGuidedResult } from '../src/lib/guided.ts';
import { StepSelectionGraph } from '../src/lib/step-selection.ts';
import type { ResultFaces } from '../src/lib/types.ts';
test('preparing correction is explicit and limited to a current native analytical result', () => {
  for (const reconstruction of ['cylinders','primitives','surfaces'] as const) {
    const result = {request_id:4,settings:{input_unit:'mm' as const,reconstruction,weld_tolerance_mm:1e-6,plane_tolerance_mm:1e-6,cylinder_fit_tolerance_mm:1e-5,max_deviation_mm:0.05}};
    assert.equal(canPrepareCorrection(result,4,reconstruction,true,false),true);
    assert.equal(canPrepareCorrection(result,5,reconstruction,true,false),false);
    assert.equal(canPrepareCorrection(result,4,reconstruction,false,false),false);
    assert.equal(canPrepareCorrection(result,4,reconstruction,true,true),false);
    assert.equal(canPrepareCorrection(result,4,'planar',true,false),false);
    assert.equal(canPrepareCorrection({...result,settings:{...result.settings,reconstruction:'planar'}},4,reconstruction,true,false),false);
    assert.equal(result.settings.reconstruction,reconstruction); // No automatic mode mutation.
  }
  assert.equal(canPrepareCorrection(null,4,'surfaces',true,false),false);
});
function fixture() {
  const document:ResultFaces = { request_id:4,source_id:2,deflection_mm:0.01,mesh:{vertices:[[0,0,0],[1,0,0],[0,1,0]],triangles:[[0,1,2],[0,1,2],[0,1,2]]},faces:[0,1,2].map(id => ({id,solid_id:id === 2 ? 1 : 0,family:'plane',triangle_start:id,triangle_count:1,adjacent_face_ids:id === 2 ? [] : [1-id]})),source_correspondence:{status:'exact',faces:[0,1,2].map(id => ({face_id:id,preview_body_id:id === 2 ? 'B' : 'A',analysis_body_id:id === 2 ? 'B/component/0' : 'A/component/0',source_triangle_ids:[id === 2 ? 0 : id]}))} };
  const graph = new StepSelectionGraph(); graph.prepare(document,{A:2,B:1});
  return {document,graph};
}
test('guidance uses exact current planar STEP face identities without accepting source triangle IDs', () => {
  const {document,graph} = fixture();
  assert.equal(guidedFaces(document,graph.inspect(),4,2),null);
  graph.pick(1,false); graph.pick(0,true);
  assert.deepEqual(guidedFaces(document,graph.inspect(),4,2),[0,1]);
  assert.equal(guidedFaces(document,graph.inspect(),5,2),null);
  assert.equal(guidedFaces(document,graph.inspect(),4,3),null);
});
test('guidance rejects multiple bodies, unavailable correspondence and accepts exact recognized surfaces', () => {
  const {document,graph} = fixture(); graph.pick(0,false); graph.pick(2,true);
  assert.equal(guidedFaces(document,graph.inspect(),4,2),null);
  graph.pick(0,false); const selection=graph.inspect();
  document.faces[0].family='cylinder'; assert.deepEqual(guidedFaces(document,selection,4,2),[0]);
  document.faces[0].family='plane'; document.source_correspondence={status:'unavailable',reason:'step.provenance_unavailable'};
  assert.equal(guidedFaces(document,selection,4,2),null);
});
test('guidance rejects ambiguous components even when the native solid list appears singular', () => {
  const {document,graph} = fixture(); graph.pick(0,false); graph.pick(1,true);
  assert.equal(document.source_correspondence.status,'exact');
  if (document.source_correspondence.status === 'exact') document.source_correspondence.faces[1].analysis_body_id='A/component/1';
  assert.equal(guidedFaces(document,graph.inspect(),4,2),null);
  const selection=graph.inspect(); selection.faces=[0,0]; assert.equal(guidedFaces(document,selection,4,2),null);
});
test('late guided success or refusal is rejected after source, options or hint removal advances revision', () => {
  assert.equal(acceptsGuidedResult(5,2,5,2,5),true);
  assert.equal(acceptsGuidedResult(5,2,6,2,5),false);
  assert.equal(acceptsGuidedResult(5,2,5,3,5),false);
  assert.equal(acceptsGuidedResult(5,2,5,null,5),false);
  assert.equal(acceptsGuidedResult(5,2,5,2,4),false);
});

test('a guided STEP retains source inspection and permits a subsequent hint on remaining facets', () => {
  const {document} = fixture();
  document.request_id = 5;
  document.faces[0].family = 'cylinder';
  const graph = new StepSelectionGraph(); graph.prepare(document,{A:2,B:1});
  const cylinder = graph.pick(0,false);
  assert.deepEqual(cylinder.source,{status:'exact',triangleCount:1,bodies:[{previewBodyId:'A',triangles:[0]}]});
  assert.deepEqual(guidedFaces(document,cylinder,5,2,1),[0]);
  const remainingFacet = graph.pick(1,false);
  assert.deepEqual(guidedFaces(document,remainingFacet,5,2,1),[1]);
  assert.equal(guidedFaces(document,remainingFacet,4,2,1),null);
  assert.deepEqual(guidedFaces(document,remainingFacet,5,2,15),[1]);
  assert.equal(guidedFaces(document,remainingFacet,5,2,16),null);
  assert.equal(guidedFaces(document,remainingFacet,5,2,-1),null);
});

test('native plane, cylinder and cone reports have distinct labels in both complete dictionaries', async () => {
  const { default: fr } = await import('../src/lib/locales/fr.ts');
  const { default: en } = await import('../src/lib/locales/en.ts');
  assert.equal(guidedFamilyLabel('plane'), 'guidance.planeLabel');
  assert.equal(guidedFamilyLabel('cone'), 'guidance.coneLabel');
  assert.equal(guidedFamilyLabel('cylinder'), 'guidance.cylinderLabel');
  assert.deepEqual(Object.keys(fr.guidance).sort(), Object.keys(en.guidance).sort());
  for (const dict of [fr, en]) {
    assert.equal(new Set([dict.guidance.planeLabel, dict.guidance.coneLabel, dict.guidance.cylinderLabel]).size, 3);
    for (const key of ['plane_incomplete','plane_residual','planeScope','plane_unproven','plane_unsupported_boundary','plane_verified','plane_applied','family','tryFamily','coneScope','cone_unproven','ambiguous_cone','unsupported_family','cone_applied'] as const) assert.ok(dict.guidance[key]);
  }
});

test('automatic results never require destructive preparation and new families retain native labels', () => {
 const settings={input_unit:'mm' as const,reconstruction:'auto' as const,weld_tolerance_mm:1e-6,plane_tolerance_mm:1e-6,cylinder_fit_tolerance_mm:1e-5,max_deviation_mm:0.05};
 assert.equal(canPrepareCorrection({request_id:4,settings},4,'auto',true,false),false);
 assert.equal(guidedFamilyLabel('auto'),'guidance.autoLabel');
 assert.equal(guidedFamilyLabel('torus'),'guidance.torusLabel');
});
