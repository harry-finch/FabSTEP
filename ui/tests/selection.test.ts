import { test } from 'node:test';
import { strict as assert } from 'node:assert';
import { SelectionGraph, SelectionReplies, SourceClickGesture, attributions } from '../src/lib/selection.ts';
import type { SelectionTopology, BodyResult } from '../src/lib/types.ts';

const pointer = (pointerId=1, clientX=20, clientY=30, button=0) => ({ pointerId,clientX,clientY,button });
test('click accepts primary stationary gesture and rejects orbit, return drag, pan, multipointer, modifier and cancel', () => {
  const gesture = new SourceClickGesture();
  gesture.down(pointer()); assert.equal(gesture.up(pointer()),true);
  gesture.down({ ...pointer(),isPrimary:false }); assert.equal(gesture.up(pointer()),false);
  gesture.down(pointer()); gesture.move(pointer(1,25)); assert.equal(gesture.up(pointer()),false);
  gesture.down(pointer(1,20,30,1)); assert.equal(gesture.up(pointer(1,20,30,1)),false);
  gesture.down({ ...pointer(),ctrlKey:true }); assert.equal(gesture.up(pointer()),false);
  gesture.down(pointer()); gesture.down(pointer(2)); assert.equal(gesture.up(pointer(2)),false); assert.equal(gesture.up(pointer()),false);
  gesture.down(pointer()); gesture.cancel(); assert.equal(gesture.up(pointer()),false);
  gesture.down(pointer()); assert.equal(gesture.up(pointer(1,25)),false);
  gesture.down(pointer()); assert.equal(gesture.up({ ...pointer(),shiftKey:true }),false);
});

test('source, unit, clear and request guards reject late work without restoring a zone', () => {
  const replies = new SelectionReplies();
  const oldFile = replies.next(7); replies.newSource(); assert.equal(replies.accepts(oldFile,7),false);
  const sameFilename = replies.next(8); replies.newSource(); assert.equal(replies.accepts(sameFilename,8),false);
  const unitChange = replies.next(9); replies.newSource(); assert.equal(replies.accepts(unitChange,9),false);
  const beforeClear = replies.next(10); replies.clear(); assert.equal(replies.accepts(beforeClear,10),false);
  const request = replies.next(11); assert.equal(replies.accepts(request,12),false);
  const firstRing = replies.next(12), secondRing = replies.next(12);
  assert.equal(replies.accepts(firstRing,12),false); assert.equal(replies.accepts(secondRing,12),true);
});

// Les composantes alternent leurs triangles dans l'aperçu ; aucune correspondance spatiale.
const topology:SelectionTopology = { request_id:4, source_id:2, components:[
  { preview_body_id:'stl', analysis_body_id:'stl/component/0', analysis_to_source_triangles:[4,0,2], source_triangle_neighbors:[[0],[2,4],[0]] },
  { preview_body_id:'stl', analysis_body_id:'stl/component/1', analysis_to_source_triangles:[1,3,5], source_triangle_neighbors:[[3],[1,5],[3]] },
  { preview_body_id:'3mf/placed/mirror', analysis_body_id:'3mf/placed/mirror/component/0', analysis_to_source_triangles:[0], source_triangle_neighbors:[[]] },
] };
function graph() { const g = new SelectionGraph(); g.prepare(topology,{ stl:6,'3mf/placed/mirror':1 }); return g; }
test('worker graph grows edge rings deterministically and shrinking restores the exact seed', () => {
  const g = graph();
  assert.deepEqual(g.zone('stl',0,[0],1,1).triangles,[0,2,4]);
  assert.deepEqual(g.zone('stl',0,[0,2,4],0,1).triangles,[0]);
  assert.equal(g.zone('stl',0,[0],2,1).hasMore,false);
  assert.deepEqual(g.zone('stl',1,[1],1,2).triangles,[1,3]);
  assert.deepEqual(g.zone('stl',1,[1,3],2,2).triangles,[1,3,5]);
  assert.deepEqual(g.zone('3mf/placed/mirror',0,[0],1,3).triangles,[0]);
});
test('new topology preserves an existing region as growth baseline and never crosses components', () => {
  const g = graph();
  assert.deepEqual(g.zone('stl',1,[1,3],1,1).triangles,[1,3,5]);
  assert.deepEqual(g.zone('stl',1,[1,3,5],0,1).triangles,[1,3]);
  // Un changement de soudage peut séparer une ancienne zone conservée en deux composantes.
  assert.deepEqual(g.zone('stl',1,[0,1],1,2).triangles,[0,1,2,3,4]);
  assert.throws(() => g.zone('stl',1,[99],1,3));
});
test('classification follows exact source mapping and placed body, never source coordinates or component numbering', () => {
  const g = graph();
  g.interpret([{ analysisBodyId:'stl/component/0', families:['cone','plane','cylinder'] }, { analysisBodyId:'stl/component/1', families:['sphere','sphere','torus'] }]);
  assert.deepEqual(g.inspect('stl',[0,4]),{ families:{ plane:1,cone:1 },unavailable:0,analysisBodyIds:['stl/component/0'] });
  assert.deepEqual(g.inspect('stl',[1,5]).families,{ sphere:1,torus:1 });
  assert.equal(g.inspect('3mf/placed/mirror',[0]).unavailable,1);
  assert.equal(g.inspect('stl',[99]).unavailable,1);
  g.interpret([{ analysisBodyId:'stl/component/0',families:['cone'] }]);
  assert.equal(g.inspect('stl',[0]).unavailable,1);
});
test('planar/extrusion attribution absent, incomplete or out of range remains unavailable', () => {
  const body = { id:'body',analysis:{ mesh:{ triangles:[[0,1,2],[1,2,3]] } },recognition:{},mixed_recognition:null,rounded_recognition:null,revolved_recognition:null } as unknown as BodyResult;
  assert.equal(attributions([body])[0].families,null);
  const mixed = { ...body,mixed_recognition:{ triangle_faces:[{ kind:'plane',index:0 }],model:{ planes:[{}],cylinders:[],partial_cylinders:[] } } } as unknown as BodyResult;
  assert.equal(attributions([mixed])[0].families,null);
  mixed.mixed_recognition!.triangle_faces = [{ kind:'plane',index:0 },{ kind:'cylinder',index:0 }];
  assert.equal(attributions([mixed])[0].families,null);
  mixed.mixed_recognition!.triangle_faces = [{ kind:'plane',index:0 },{ kind:'plane',index:0 }];
  assert.deepEqual(attributions([mixed])[0].families,['plane','plane']);
});
test('invalid adjacency, duplicate ownership, missing source coverage and work budgets fail closed', () => {
  const invalid = structuredClone(topology); invalid.components[0].source_triangle_neighbors[0] = [1];
  assert.throws(() => new SelectionGraph().prepare(invalid,{ stl:6,'3mf/placed/mirror':1 }));
  const duplicate = structuredClone(topology); duplicate.components[1].analysis_to_source_triangles[0] = 0;
  assert.throws(() => new SelectionGraph().prepare(duplicate,{ stl:6,'3mf/placed/mirror':1 }));
  assert.throws(() => new SelectionGraph().prepare(topology,{ stl:7,'3mf/placed/mirror':1 }));
  assert.throws(() => new SelectionGraph().prepare(topology,{ stl:100001,'3mf/placed/mirror':1 }));
});

test('actual worker queues preparation, interpretation and rings with revision guards', async (t) => {
  const { Worker } = await import('node:worker_threads');
  const workerUrl = new URL('../src/lib/selection.worker.ts',import.meta.url).href;
  const worker = new Worker(`const { parentPort } = require('node:worker_threads'); globalThis.self = { postMessage:value => parentPort.postMessage(value), set onmessage(handler) { parentPort.on('message',data => handler({ data })); } }; import(${JSON.stringify(workerUrl)});`,{ eval:true });
  t.after(() => worker.terminate());
  const next = () => new Promise<any>((resolve,reject) => { worker.once('message',resolve); worker.once('error',reject); });
  const token = { revision:2,requestId:4,epoch:1,job:1 };
  let response = next(); worker.postMessage({ type:'prepare',token,topology,bodyCounts:{ stl:6,'3mf/placed/mirror':1 } });
  assert.equal((await response).type,'ready');
  response = next(); worker.postMessage({ type:'interpret',token,items:[{ analysisBodyId:'stl/component/0',families:['partial_cone','plane','cylinder'] }] });
  assert.equal((await response).type,'interpreted');
  // Une ancienne source est ignorée ; la requête courante sert aussi de barrière FIFO.
  worker.postMessage({ type:'zone',token:{ ...token,revision:1 },bodyId:'stl',seed:1,triangles:[1],ring:2 });
  response = next(); worker.postMessage({ type:'zone',token:{ ...token,job:2 },bodyId:'stl',seed:0,triangles:[0],ring:1 });
  const grown = await response;
  assert.equal(grown.token.job,2); assert.deepEqual(grown.triangles,[0,2,4]); assert.deepEqual(grown.inspection.families,{ plane:1,cylinder:1,partial_cone:1 });
  response = next(); worker.postMessage({ type:'prepare',token:{ ...token,job:3 },topology,bodyCounts:{ stl:7 } });
  assert.equal((await response).type,'unavailable');
});


test('partial conical bands retain exact source attribution and reject absent or invalid model references', () => {
  const body = {
    id:'stl/component/0', analysis:{mesh:{triangles:[[0,1,2],[2,3,0],[3,4,5]]}},
    rounded_recognition:{
      model:{revolved:{mixed:{planes:[],cylinders:[],partial_cylinders:[{arcs:[0,1],inner:true}]},cones:[]},
        partial_cones:[{arcs:[1,2],inner:true}],spheres:[],tori:[]},
      triangle_faces:[{kind:'partial_cone',index:0},{kind:'partial_cylinder',index:0},{kind:'partial_cone',index:0}]
    }
  } as unknown as BodyResult;
  const items = attributions([body]);
  assert.deepEqual(items[0].families,['partial_cone','partial_cylinder','partial_cone']);
  const g = graph();
  g.interpret(items);
  // Source indices are deliberately reordered: source 0 belongs to the cylinder.
  assert.deepEqual(g.inspect('stl',[4,0,2]),{
    families:{partial_cone:2,partial_cylinder:1},unavailable:0,analysisBodyIds:['stl/component/0']
  });
  body.rounded_recognition!.triangle_faces[0].index=1;
  assert.equal(attributions([body])[0].families,null);
  body.rounded_recognition!.triangle_faces[0].index=0;
  delete body.rounded_recognition!.model.partial_cones;
  assert.equal(attributions([body])[0].families,null);
  // Older reports without the additive field still classify their existing families.
  body.rounded_recognition!.triangle_faces=Array.from({length:3},()=>({kind:'partial_cylinder' as const,index:0}));
  assert.deepEqual(attributions([body])[0].families,['partial_cylinder','partial_cylinder','partial_cylinder']);
});


test('spherical octants keep source inspection complete and require a valid additive reference', () => {
  const body = {
    id:'stl/component/0', analysis:{mesh:{triangles:[[0,1,2],[2,3,0],[3,4,5]]}},
    rounded_recognition:{
      model:{revolved:{mixed:{planes:[{}],cylinders:[],partial_cylinders:[{}]},cones:[]},
        spheres:[],tori:[],octants:[{}]},
      triangle_faces:[{kind:'spherical_octant',index:0},{kind:'partial_cylinder',index:0},{kind:'plane',index:0}]
    }
  } as unknown as BodyResult;
  const items = attributions([body]);
  assert.deepEqual(items[0].families,['spherical_octant','partial_cylinder','plane']);
  const g = graph();
  g.interpret(items);
  assert.deepEqual(g.inspect('stl',[4,0,2]),{
    families:{spherical_octant:1,partial_cylinder:1,plane:1},unavailable:0,analysisBodyIds:['stl/component/0']
  });
  body.rounded_recognition!.triangle_faces[0].index=1;
  assert.equal(attributions([body])[0].families,null);
  body.rounded_recognition!.triangle_faces[0].index=0;
  delete body.rounded_recognition!.model.octants;
  assert.equal(attributions([body])[0].families,null);
  body.rounded_recognition!.triangle_faces[0]={kind:'plane',index:0};
  assert.deepEqual(attributions([body])[0].families,['plane','partial_cylinder','plane']);
});


test('linear blends retain their conical owner and reject missing or invalid model references', () => {
  const body = {
    id:'stl/component/0',analysis:{mesh:{triangles:[[0,1,2],[2,3,0],[3,4,5]]}},
    rounded_recognition:{
      model:{revolved:{mixed:{planes:[{}],cylinders:[],partial_cylinders:[]},cones:[]},
        spheres:[],tori:[],linear_blend:{}},
      triangle_faces:[{kind:'linear_blend',index:0},{kind:'plane',index:0},{kind:'linear_blend',index:0}]
    }
  } as unknown as BodyResult;
  const items=attributions([body]);
  assert.deepEqual(items[0].families,['linear_blend','plane','linear_blend']);
  const g=graph();g.interpret(items);
  assert.deepEqual(g.inspect('stl',[4,0,2]),{
    families:{linear_blend:2,plane:1},unavailable:0,analysisBodyIds:['stl/component/0']
  });
  body.rounded_recognition!.triangle_faces[0].index=1;
  assert.equal(attributions([body])[0].families,null);
  body.rounded_recognition!.triangle_faces[0].index=0;
  delete body.rounded_recognition!.model.linear_blend;
  assert.equal(attributions([body])[0].families,null);
  body.rounded_recognition!.triangle_faces=Array.from({length:3},()=>({kind:'plane' as const,index:0}));
  assert.deepEqual(attributions([body])[0].families,['plane','plane','plane']);
});


test('cross-drill attribution bounds all three cylinder domains and preserves legacy models', () => {
  const body = {
    id:'stl/component/0',analysis:{mesh:{triangles:[[0,1,2],[2,3,0],[3,4,5]]}},
    rounded_recognition:{
      model:{revolved:{mixed:{planes:[{}],cylinders:[],partial_cylinders:[]},cones:[]},
        spheres:[],tori:[],cross_drill:{}},
      triangle_faces:[{kind:'cross_drill',index:0},{kind:'cross_drill',index:1},{kind:'cross_drill',index:2}]
    }
  } as unknown as BodyResult;
  const items=attributions([body]);
  assert.deepEqual(items[0].families,['cross_drill','cross_drill','cross_drill']);
  const g=graph();g.interpret(items);
  assert.deepEqual(g.inspect('stl',[4,0,2]),{
    families:{cross_drill:3},unavailable:0,analysisBodyIds:['stl/component/0']
  });
  body.rounded_recognition!.triangle_faces[2].index=3;
  assert.equal(attributions([body])[0].families,null);
  body.rounded_recognition!.triangle_faces[2].index=2;
  delete body.rounded_recognition!.model.cross_drill;
  assert.equal(attributions([body])[0].families,null);
  body.rounded_recognition!.triangle_faces=Array.from({length:3},()=>({kind:'plane' as const,index:0}));
  assert.deepEqual(attributions([body])[0].families,['plane','plane','plane']);
});
