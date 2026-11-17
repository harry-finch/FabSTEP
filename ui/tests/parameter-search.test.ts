import { test } from 'node:test';
import { strict as assert } from 'node:assert';
import { ParameterSearchReplies, proposedTolerance, type ParameterSearchReport } from '../src/lib/parameter-search.ts';
import type { Options } from '../src/lib/types.ts';
const options:Options = {input_unit:'mm',reconstruction:'surfaces',weld_tolerance_mm:1e-6,plane_tolerance_mm:1e-6,cylinder_fit_tolerance_mm:1e-5,max_deviation_mm:0.05};
test('search rejects stale sources, revisions, invocations, options and invalidation', () => {
  const replies = new ParameterSearchReplies();
  const first = replies.begin(10,3,options);
  assert.equal(replies.accepts(first,10,3,options,10),true);
  assert.equal(replies.accepts(first,11,3,options),false);
  assert.equal(replies.accepts(first,10,4,options),false);
  assert.equal(replies.accepts(first,10,3,options,9),false);
  for (const patch of [{input_unit:'inch'}, {max_deviation_mm:0.1}, {cylinder_fit_tolerance_mm:2e-5}, {reconstruction:'planar'}, {weld_tolerance_mm:1e-5}]) assert.equal(replies.accepts(first,10,3,{...options,...patch} as Options),false);
  const second = replies.begin(10,3,options);
  assert.equal(replies.accepts(first,10,3,options),false);
  assert.equal(replies.accepts(second,10,3,options),true);
  replies.clear();
  assert.equal(replies.accepts(second,10,3,options),false);
});
test('apply extracts only a validated finite proposal, without modifying other settings', () => {
  const report:ParameterSearchReport = {settings:{...options},best_index:0,trials:[{cylinder_fit_tolerance_mm:1.2e-5,outcome:'validated',faces:123,curved_faces:25,max_deviation_bound_mm:0.009,max_sampled_distance_mm:0.008,error:null}]};
  assert.equal(proposedTolerance(report),1.2e-5);
  const changed = {...options,cylinder_fit_tolerance_mm:proposedTolerance(report)!};
  assert.equal(changed.max_deviation_mm,options.max_deviation_mm);
  for (const best_index of [null,-1,0.5,2]) assert.equal(proposedTolerance({...report,best_index}),null);
  for (const value of [NaN,Infinity,0,-1]) assert.equal(proposedTolerance({...report,trials:[{...report.trials[0],cylinder_fit_tolerance_mm:value}]}),null);
  assert.equal(proposedTolerance({...report,trials:[{...report.trials[0],outcome:'rejected'}]}),null);
});
