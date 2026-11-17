import { test } from 'node:test';
import { strict as assert } from 'node:assert';
import { createFaceProfileGeometryWorker, type FaceProfileWorker } from '../src/lib/face-profile-worker.ts';
import { faceProfileGeometry } from '../src/lib/face-profile-geometry.ts';
import type { PlanarFaceProfile } from '../src/lib/types.ts';
const profile:PlanarFaceProfile={face_id:0,solid_id:0,frame:{origin_mm:[0,0,0],x_axis:[1,0,0],y_axis:[0,1,0],normal:[0,0,1]},loops:[{is_outer:true,segments:[{kind:'circle',center_mm:[5,8],radius_mm:3}]}]};
function fixture(){
  const sent:{id:number;profile:PlanarFaceProfile}[]=[];let terminated=0;
  const port={postMessage:(message:typeof sent[number])=>sent.push(message),terminate:()=>{terminated++;},onmessage:null,onerror:null} as unknown as FaceProfileWorker;
  const worker=createFaceProfileGeometryWorker(port);
  const reply=(id:number,geometry=faceProfileGeometry(profile))=>port.onmessage?.({data:{id,geometry}} as MessageEvent);
  return {worker,port,sent,reply,terminated:()=>terminated};
}
test('display worker queue retains only one active and one latest profile',async()=>{
  const f=fixture(),first=f.worker.prepare(profile).catch(error=>error),middle=f.worker.prepare(profile).catch(error=>error),last=f.worker.prepare(profile);
  assert.equal((await first).code,'engine.stale');assert.equal((await middle).code,'engine.stale');assert.equal(f.sent.length,1);
  f.reply(1);assert.equal(f.sent.length,2);assert.equal(f.sent[1].id,3);f.reply(1);f.reply(3);
  assert.deepEqual((await last).bounds,[[2,5],[8,11]]);f.worker.dispose();
});
test('clear rejects queued work and a later request waits for active worker completion',async()=>{
  const f=fixture(),old=f.worker.prepare(profile).catch(error=>error),queued=f.worker.prepare(profile).catch(error=>error);f.worker.clear();
  assert.equal((await old).code,'engine.stale');assert.equal((await queued).code,'engine.stale');const latest=f.worker.prepare(profile);assert.equal(f.sent.length,1);
  f.reply(1);assert.equal(f.sent[1].id,3);f.reply(3);assert.ok(await latest);f.worker.dispose();
});
test('worker error and dispose reject pending work and forbid future preparation',async()=>{
  for(const error of [false,true]){const f=fixture(),active=f.worker.prepare(profile).catch(error=>error),queued=f.worker.prepare(profile).catch(error=>error);
    if(error)f.port.onerror?.({} as ErrorEvent);else f.worker.dispose();
    assert.equal((await active).code,'engine.stale');assert.equal((await queued).code,error?'engine.task':'engine.stale');assert.equal(f.terminated(),1);await assert.rejects(f.worker.prepare(profile),(e:{code:string})=>e.code==='engine.task');}
});
test('preview limits reject before structured clone or worker dispatch',async()=>{
  const f=fixture(),tooMany=structuredClone(profile);tooMany.loops=Array.from({length:1025},()=>profile.loops[0]);await assert.rejects(f.worker.prepare(tooMany),(e:{code:string})=>e.code==='dxf.too_complex');
  tooMany.loops=[{is_outer:true,segments:Array.from({length:16385},()=>profile.loops[0].segments[0])}];await assert.rejects(f.worker.prepare(tooMany),(e:{code:string})=>e.code==='dxf.too_complex');assert.equal(f.sent.length,0);f.worker.dispose();
});
test('actual profile worker returns analytic geometry and stable refusals',async t=>{
  const {Worker}=await import('node:worker_threads'), url=new URL('../src/lib/face-profile.worker.ts',import.meta.url).href;
  const worker=new Worker(`const {parentPort}=require('node:worker_threads');globalThis.self={postMessage:value=>parentPort.postMessage(value),set onmessage(handler){parentPort.on('message',data=>handler({data}));}};import(${JSON.stringify(url)});`,{eval:true});t.after(()=>worker.terminate());
  const command=(message:unknown)=>new Promise<any>((resolve,reject)=>{worker.once('message',resolve);worker.once('error',reject);worker.postMessage(message);});
  const reply=await command({id:8,profile});assert.equal(reply.id,8);assert.deepEqual(reply.geometry.bounds,[[2,5],[8,11]]);assert.match(reply.geometry.paths[0].d,/A 3 3/);
  const bad=structuredClone(profile);bad.loops=[];const refused=await command({id:9,profile:bad});assert.equal(refused.id,9);assert.equal(refused.error.code,'dxf.invalid_profile');
});
