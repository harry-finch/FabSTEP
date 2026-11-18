import { test } from 'node:test';
import { strict as assert } from 'node:assert';
import { visibleDiagnostic, triangleLimitCounts } from '../src/lib/diagnostics.ts';
import type { Diagnostic } from '../src/lib/types.ts';

const notice = (code:string, severity:Diagnostic['severity'] = 'warning'):Diagnostic => ({code,severity,message:'Native detail',count:1});

test('explicit routine settings and recognition intent stay in technical details', () => {
  for (const code of ['precision.custom_tolerances', 'recognition.circular_intent_assumed', 'recognition.conic_intent_assumed', 'recognition.rounded_intent_assumed', 'recognition.oblique_pocket_fitted', 'recognition.adjacent_pocket_fitted', 'recognition.rounded_box_intent_assumed']) {
    assert.equal(visibleDiagnostic(notice(code)),false,code);
  }
  for (const code of ['auto.faceted_base','auto.planar_regions_split','auto.regions_faceted','guidance.auto_applied','guidance.torus_applied','validation.native_required','recognition.rounded_coverage','recognition.bound_scope','document.separate_bodies','guidance.cylinder_applied','guidance.cone_applied','guidance.plane_applied','guidance.plane_verified','guidance.complete_cylinder_verified','guidance.complete_cone_verified']) {
    assert.equal(visibleDiagnostic(notice(code,'info')),false,code);
    assert.equal(visibleDiagnostic(notice(code)),true,`${code} escalated to warning`);
  }
});

test('geometry changes, discarded source metadata and unknown codes remain visible', () => {
  for (const code of ['recognition.bored_prism_projection','recognition.linear_blend_endpoints_projected','recognition.oblique_pocket_adjusted','recognition.adjacent_pocket_adjusted','guidance.plane_residual','mesh.vertices_moved','mesh.orientation_reversed','planar.approximate','threemf.metadata_ignored','future.geometry_changed','recognition.future_intent_assumed']) {
    for (const severity of ['info','warning'] as const) assert.equal(visibleDiagnostic(notice(code,severity)),true,`${code}: ${severity}`);
  }
});

test('future blocking severity is never hidden by a known routine code', () => {
  for (const code of ['precision.custom_tolerances','recognition.rounded_intent_assumed','validation.native_required']) {
    assert.equal(visibleDiagnostic({...notice(code),severity:'error'} as unknown as Diagnostic),true);
  }
});


test('STL limit diagnostics expose exact counts without inventing values for legacy or malformed errors',()=>{
  assert.deepEqual(triangleLimitCounts('input.too_many_triangles','STL : 540170 triangles ; limite : 100000.'),{count:540170,limit:100000});
  for(const message of ['STL limité à 100 000 triangles.','Maillage limité à 100 000 triangles et 300 000 sommets.','STL : 1 triangles ; limite : 100000.','STL : 100001 triangles ; limite : 0.','STL : 9007199254740992 triangles ; limite : 100000.',null]) assert.equal(triangleLimitCounts('input.too_many_triangles',message),null);
  assert.equal(triangleLimitCounts('stl.invalid_length','STL : 540170 triangles ; limite : 100000.'),null);
});
