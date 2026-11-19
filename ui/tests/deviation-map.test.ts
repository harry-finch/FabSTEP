import assert from 'node:assert/strict';
import { test } from 'node:test';
import { Color } from 'three';
import { createDeviationMap, type DeviationReport, type DeviationState } from '../src/lib/deviation-map.ts';
import { writeDeviationColors } from '../src/lib/deviation-colors.ts';
import type { ResultFaces } from '../src/lib/types.ts';

const document = {request_id:4, source_id:7, mesh:{vertices:[[0,0,0],[1,0,0],[0,1,0]], triangles:[[0,1,2]]}, faces:[{id:0}]} as ResultFaces;
const report:DeviationReport = {vertex_distances_mm:[0,0.025,0.08], face_maxima_mm:[0.08], min_mm:0,max_mm:0.08,budget_mm:0.05};
function setup() {
  const states:DeviationState[] = [];
  const controller = createDeviationMap(state => states.push(state));
  const calls:{name:string; args:Record<string,unknown>}[] = [];
  let finish!:(report:unknown)=>void, fail!:(error:unknown)=>void;
  const bridge = {invoke:<T>(name:string,args:Record<string,unknown>):Promise<T> => {
    calls.push({name,args});
    if (name === 'cancel_deviation_map') return Promise.resolve(undefined as T);
    return new Promise<T>((resolve,reject) => { finish = value => resolve(value as T); fail = reject; });
  }};
  return {states,controller,calls,bridge,reply:(value:unknown)=>finish(value),fail:(error:unknown)=>fail(error)};
}
test('publishes distances only for this document, with complete finite arrays', async()=>{
  for (const variant of ['valid','source','request','epoch','missing','nan','range']) {
    const s = setup(), pending = s.controller.load(s.bridge,document,()=>true);
    const reply = {request_id:4,source_id:7,map_id:s.calls[0].args.mapId,report:{...report}};
    if (variant === 'source') reply.source_id++;
    if (variant === 'request') reply.request_id++;
    if (variant === 'epoch') reply.map_id = -1;
    if (variant === 'missing') reply.report.vertex_distances_mm = [];
    if (variant === 'nan') reply.report.face_maxima_mm = [NaN];
    if (variant === 'range') reply.report.max_mm = -1;
    s.reply(reply); await pending;
    assert.equal(s.states.at(-1)?.kind, variant === 'valid' ? 'ready' : 'unavailable');
  }
});
test('turning the map off cancels only its own work and ignores a late reply', async()=>{
  const s = setup(), pending = s.controller.load(s.bridge,document,()=>true);
  s.controller.clear();
  assert.deepEqual(s.calls[1],{name:'cancel_deviation_map', args:s.calls[0].args});
  s.reply({request_id:4,source_id:7,map_id:s.calls[0].args.mapId,report}); await pending;
  assert.equal(s.states.at(-1)?.kind,'off');
});
test('budget refusal is local to the visualization', async()=>{
  const s = setup(), pending = s.controller.load(s.bridge,document,()=>true);
  s.fail({code:'validation.distance_budget'}); await pending;
  assert.equal(s.states.at(-1)?.kind,'overBudget');
  assert.deepEqual(s.calls.map(call=>call.name),['result_deviation_map']);
});
test('indexed vertices color the correct displayed corners, saturating at the active threshold',()=>{
  const stops = {'--view-deviation-low':'#334477','--view-deviation-mid':'#44aa99','--view-deviation-high':'#ffbb33'};
  const css = {getPropertyValue:(key:keyof typeof stops)=>stops[key]} as CSSStyleDeclaration;
  const colors = new Float32Array(9);
  writeDeviationColors(colors,[[2,0,1]],report,css);
  const expected = ['high','low','mid'].flatMap(stop =>
    new Color(stops[('--view-deviation-'+stop) as keyof typeof stops]).toArray());
  assert.deepEqual(colors,new Float32Array(expected));
});
