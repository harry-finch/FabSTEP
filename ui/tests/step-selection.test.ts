import { test } from 'node:test';
import { strict as assert } from 'node:assert';
import { StepSelectionGraph } from '../src/lib/step-selection.ts';
import { SelectionReplies, SourceClickGesture } from '../src/lib/selection.ts';
import type { ResultFaces } from '../src/lib/types.ts';
const document:ResultFaces = { request_id:9,source_id:4,deflection_mm:0.01,source_correspondence:{ status:'unavailable',reason:'step.provenance_unavailable' },mesh:{ vertices:[[0,0,0],[1,0,0],[0,1,0]],triangles:Array.from({length:5},() => [0,1,2]) },faces:[
  { id:0,solid_id:0,family:'plane',triangle_start:0,triangle_count:2,adjacent_face_ids:[1] },
  { id:1,solid_id:0,family:'cylinder',triangle_start:2,triangle_count:1,adjacent_face_ids:[0,2] },
  { id:2,solid_id:0,family:'cone',triangle_start:3,triangle_count:1,adjacent_face_ids:[1] },
  { id:3,solid_id:1,family:'sphere',triangle_start:4,triangle_count:1,adjacent_face_ids:[] }
] };
function graph() { const graph = new StepSelectionGraph(); graph.prepare(document); return graph; }
test('native face selection covers every tessellated triangle and modifier toggles face identities', () => {
  const g = graph(); assert.deepEqual(g.pick(0,false).triangles,[0,1]);
  assert.deepEqual(g.pick(3,true).families,{ plane:1,sphere:1 });
  assert.deepEqual(g.inspect().solids,[0,1]);
  assert.deepEqual(g.pick(0,true).faces,[3]);
  assert.deepEqual(g.pick(1,false).faces,[1]);
  assert.deepEqual(g.clear().triangles,[]);
  assert.throws(() => g.pick(1.5,false)); assert.throws(() => g.pick(4,false));
});
test('native adjacency grows one ring, never crosses solids, and reduction restores exact multiselection', () => {
  const g = graph(); g.pick(0,false); g.pick(3,true);
  assert.deepEqual(g.grow().faces,[0,1,3]); assert.deepEqual(g.grow().faces,[0,1,2,3]);
  assert.equal(g.inspect().hasMore,false); g.grow();
  assert.deepEqual(g.shrink().faces,[0,1,3]); assert.deepEqual(g.shrink().faces,[0,3]);
  assert.equal(g.inspect().canShrink,false);
  g.pick(1,false); assert.deepEqual(g.grow().faces,[0,1,2]);
});
test('native preview rejects incomplete ranges, unknown surface kinds, cross-solid or asymmetric adjacency and budgets', () => {
  const change = (mutate:(d:ResultFaces)=>void) => { const d=structuredClone(document); mutate(d); assert.throws(() => new StepSelectionGraph().prepare(d)); };
  change(d => d.faces[1].triangle_start++); change(d => d.faces[3].triangle_count=2);
  change(d => d.faces[0].adjacent_face_ids=[3]); change(d => d.faces[0].adjacent_face_ids=[]);
  change(d => d.faces[0].adjacent_face_ids=[1,1]); change(d => d.faces[0].solid_id=32);
  change(d => d.faces[0].family='bspline' as any); change(d => d.mesh.vertices[0][0]=Infinity);
  change(d => d.mesh.triangles[0]=[0,1,100]); change(d => d.mesh.triangles=Array(100001).fill([0,1,2]));
  change(d => d.mesh.vertices=Array(300001).fill([0,0,0]));
});
test('STEP modifier click still rejects orbit, alternate buttons and cancelled geometry gestures', () => {
  const gesture = new SourceClickGesture();
  const p = { pointerId:1,clientX:10,clientY:10,button:0,shiftKey:true };
  gesture.down(p,true); assert.equal(gesture.up(p,true),true);
  gesture.down(p,true); gesture.move({...p,clientX:20}); assert.equal(gesture.up(p,true),false);
  gesture.down(p,true); gesture.cancel(); assert.equal(gesture.up(p,true),false);
  gesture.down({...p,altKey:true},true); assert.equal(gesture.up(p,true),false);
  gesture.down({...p,button:2},true); assert.equal(gesture.up({...p,button:2},true),false);
});
test('STEP clear, view cancellation and conversion invalidation reject late prepare or selection', () => {
  const replies = new SelectionReplies();
  const preparation = replies.next(9); replies.clear(); assert.equal(replies.accepts(preparation,9),false);
  const pick = replies.next(9); replies.clear(); assert.equal(replies.accepts(pick,9),false);
  const grow = replies.next(9); replies.newSource(); assert.equal(replies.accepts(grow,10),false);
});
test('actual STEP worker validates then selects and ignores outdated result identity', async t => {
  const { Worker } = await import('node:worker_threads');
  const workerUrl = new URL('../src/lib/step-selection.worker.ts',import.meta.url).href;
  const worker = new Worker(`const { parentPort } = require('node:worker_threads'); globalThis.self = { postMessage:value => parentPort.postMessage(value), set onmessage(handler) { parentPort.on('message',data => handler({ data })); } }; import(${JSON.stringify(workerUrl)});`,{eval:true});
  t.after(() => worker.terminate());
  const next = () => new Promise<any>((resolve,reject) => { worker.once('message',resolve); worker.once('error',reject); });
  const token = { revision:2,requestId:9,epoch:0,job:1 };
  let reply=next(); worker.postMessage({type:'prepare',document,token}); assert.equal((await reply).type,'ready');
  worker.postMessage({type:'pick',id:3,additive:false,token:{...token,revision:1}});
  reply=next(); worker.postMessage({type:'pick',id:0,additive:false,token:{...token,job:2}}); assert.deepEqual((await reply).selection.triangles,[0,1]);
  reply=next(); worker.postMessage({type:'grow',token:{...token,job:3}}); assert.deepEqual((await reply).selection.faces,[0,1]);
  reply=next(); worker.postMessage({type:'select',ids:[2,1],token:{...token,job:4}}); assert.deepEqual((await reply).selection.faces,[1,2]);
  reply=next(); worker.postMessage({type:'clear',token:{...token,epoch:1,job:5}}); assert.deepEqual((await reply).selection.faces,[]);
});

const bodyCounts = { 'placed-A':3, 'placed-B':5 };
function exactDocument():ResultFaces {
  return { ...structuredClone(document), source_correspondence:{ status:'exact', faces:[
    {face_id:3,preview_body_id:'placed-A',analysis_body_id:'component-A',source_triangle_ids:[0,1,2]},
    {face_id:1,preview_body_id:'placed-B',analysis_body_id:'component-B',source_triangle_ids:[0]},
    {face_id:0,preview_body_id:'placed-B',analysis_body_id:'component-B',source_triangle_ids:[2,4]},
    {face_id:2,preview_body_id:'placed-B',analysis_body_id:'component-B',source_triangle_ids:[1,3]}
  ] } };
}
test('exact source union follows explicit face and placed-body identities across multiple solids', () => {
  const g = new StepSelectionGraph(); g.prepare(exactDocument(),bodyCounts);
  assert.deepEqual(g.inspect().source,{status:'exact',triangleCount:0,bodies:[]});
  assert.deepEqual(g.pick(0,false).source,{status:'exact',triangleCount:2,bodies:[{previewBodyId:'placed-B',triangles:[2,4]}]});
  const selected = g.pick(3,true);
  assert.deepEqual(selected.source,{status:'exact',triangleCount:5,bodies:[{previewBodyId:'placed-B',triangles:[2,4]},{previewBodyId:'placed-A',triangles:[0,1,2]}]});
  assert.deepEqual(g.inspect(),selected); // Navigation reads the same selection without issuing a clear.
  assert.deepEqual(g.grow().source,{status:'exact',triangleCount:6,bodies:[{previewBodyId:'placed-B',triangles:[0,2,4]},{previewBodyId:'placed-A',triangles:[0,1,2]}]});
  assert.deepEqual(g.shrink().source,selected.source);
  assert.deepEqual(g.clear().source,{status:'exact',triangleCount:0,bodies:[]});
});
test('incomplete, ambiguous, out-of-range or excessive provenance disables only source correspondence', () => {
  const invalid = (mutate:(d:ResultFaces)=>void, counts=bodyCounts) => {
    const d = exactDocument(); mutate(d);
    const g = new StepSelectionGraph(); g.prepare(d,counts);
    assert.deepEqual(g.pick(0,false).triangles,[0,1]);
    assert.deepEqual(g.inspect().source,{status:'unavailable',reason:'step.provenance_invalid'});
  };
  const rows = (d:ResultFaces) => { assert.equal(d.source_correspondence.status,'exact'); return d.source_correspondence.status === 'exact' ? d.source_correspondence.faces : []; };
  invalid(d => rows(d).pop());
  invalid(d => rows(d)[1].face_id=3);
  invalid(d => rows(d)[1].face_id=4);
  invalid(d => rows(d)[0].source_triangle_ids=[]);
  invalid(d => rows(d)[0].source_triangle_ids=[0,1,3]);
  invalid(d => rows(d)[0].source_triangle_ids=[0,1,1]);
  invalid(d => rows(d)[0].source_triangle_ids=[2,1,0]);
  invalid(d => rows(d)[1].source_triangle_ids=[2]);
  invalid(d => rows(d)[1].source_triangle_ids=[-1]);
  invalid(d => rows(d)[1].source_triangle_ids=[0.5]);
  invalid(d => rows(d)[1].source_triangle_ids=[NaN]);
  invalid(d => rows(d)[1].preview_body_id='solid-0');
  invalid(d => rows(d)[1].analysis_body_id='');
  invalid(d => rows(d)[1].analysis_body_id='component-A');
  invalid(() => {},{});
  invalid(() => {},{...bodyCounts,'placed-B':6});
  invalid(() => {},{...bodyCounts,'placed-B':100_001});
  invalid(d => rows(d)[0].source_triangle_ids=Array(100_001).fill(0));
  invalid(d => (d as any).source_correspondence={status:'exact',faces:null});
  invalid(d => (d as any).source_correspondence=null);
});
test('unavailable modes and a new preparation cannot retain an earlier exact source attribution', () => {
  const g = new StepSelectionGraph(); g.prepare(exactDocument(),bodyCounts); g.pick(0,false);
  const unsupported = structuredClone(document);
  unsupported.source_correspondence={status:'unavailable',reason:'step.provenance_unavailable'};
  g.prepare(unsupported,bodyCounts);
  assert.deepEqual(g.pick(0,false).source,{status:'unavailable',reason:'step.provenance_unavailable'});
  assert.deepEqual(g.inspect().faces,[0]);
  g.prepare(exactDocument(),bodyCounts);
  assert.deepEqual(g.inspect().faces,[]);
  assert.deepEqual(g.inspect().source,{status:'exact',triangleCount:0,bodies:[]});
});
test('actual STEP worker maps source bodies, clears derived triangles and accepts invalid provenance for native inspection', async t => {
  const { Worker } = await import('node:worker_threads');
  const workerUrl = new URL('../src/lib/step-selection.worker.ts',import.meta.url).href;
  const worker = new Worker(`const { parentPort } = require('node:worker_threads'); globalThis.self = { postMessage:value => parentPort.postMessage(value), set onmessage(handler) { parentPort.on('message',data => handler({ data })); } }; import(${JSON.stringify(workerUrl)});`,{eval:true});
  t.after(() => worker.terminate());
  const replies = new SelectionReplies();
  const command = (message:any) => new Promise<any>((resolve,reject) => { worker.once('message',resolve); worker.once('error',reject); worker.postMessage(message); });
  let reply = await command({type:'prepare',document:exactDocument(),bodyCounts,token:replies.next(9)});
  assert.equal(reply.type,'ready'); assert.equal(reply.selection.source.status,'exact');
  reply = await command({type:'pick',id:0,additive:false,token:replies.next(9)});
  assert.deepEqual(reply.selection.source.bodies,[{previewBodyId:'placed-B',triangles:[2,4]}]);
  reply = await command({type:'pick',id:3,additive:true,token:replies.next(9)});
  assert.equal(reply.selection.source.bodies.length,2);
  const late = reply; replies.clear();
  reply = await command({type:'clear',token:replies.next(9)});
  assert.deepEqual(reply.selection.source.bodies,[]);
  assert.equal(replies.accepts(late.token,9),false);
  assert.equal(replies.accepts(reply.token,9),true);
  const invalid = exactDocument(); invalid.source_correspondence={status:'exact',faces:[]};
  reply = await command({type:'prepare',document:invalid,bodyCounts,token:replies.next(9)});
  assert.equal(reply.type,'ready'); assert.equal(reply.selection.source.reason,'step.provenance_invalid');
  reply = await command({type:'pick',id:3,additive:false,token:replies.next(9)});
  assert.deepEqual(reply.selection.faces,[3]);
  replies.newSource(); assert.equal(replies.accepts(reply.token,10),false);
});

test('a suggested wall replaces selection atomically without growing across unrelated faces', () => {
 const g=graph();g.pick(3,false);
 assert.deepEqual(g.select([2,1]).faces,[1,2]);
 for(const invalid of [[],[0,0],[0,3],[9],[-1],[0.5]]) {
  assert.throws(()=>g.select(invalid));assert.deepEqual(g.inspect().faces,[1,2]);
 }
 assert.equal(g.inspect().canShrink,false);
});

test('paint batches add visible face IDs without toggles and reject invalid batches atomically',()=>{
  const g=graph();g.pick(0,false);
  assert.deepEqual(g.add([1,2,1]).faces,[0,1,2]);
  assert.deepEqual(g.add([1,2]).faces,[0,1,2]);
  assert.throws(()=>g.add([3,99]));assert.deepEqual(g.inspect().faces,[0,1,2]);
  g.clear();assert.deepEqual(g.add([2]).faces,[2]);
});


test('certified revolution faces keep their own STEP family through selection and adjacency', () => {
  const d=structuredClone(document);d.faces[1].family='revolution';
  const g=new StepSelectionGraph();g.prepare(d);
  assert.deepEqual(g.pick(1,false).families,{revolution:1});
  assert.deepEqual(g.grow().families,{plane:1,revolution:1,cone:1});
  assert.deepEqual(g.shrink().families,{revolution:1});
});
