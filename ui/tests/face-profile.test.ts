import { test } from 'node:test';
import { strict as assert } from 'node:assert';
import { createFaceProfiles, type FaceProfileContext, type FaceProfileState } from '../src/lib/face-profile.ts';
import type { FaceProfileResponse, PlanarFaceProfile } from '../src/lib/types.ts';
import type { SourceBridge } from '../src/lib/source.ts';
import { faceProfileGeometry } from '../src/lib/face-profile-geometry.ts';

function deferred<T>() { let resolve!:(value:T)=>void, reject!:(error:unknown)=>void; const promise = new Promise<T>((yes,no) => { resolve=yes; reject=no; }); return {resolve,reject,promise}; }
const tick = async () => { for (let i=0;i<12;i++) await Promise.resolve(); };
const plane = (faceId=2):PlanarFaceProfile => ({ face_id:faceId,solid_id:0,frame:{origin_mm:[0,0,0],x_axis:[1,0,0],y_axis:[0,1,0],normal:[0,0,1]},loops:[{is_outer:true,segments:[{kind:'circle',center_mm:[0,0],radius_mm:4}]}] });
function fixture(initialSelectionId=0) {
  const state:FaceProfileState = {response:null,geometry:null,pending:false,exporting:false,error:null,note:null};
  let context:FaceProfileContext|null = {requestId:1,sourceId:9,faceId:2};
  const calls:{command:string;args:Record<string,unknown>;pending:ReturnType<typeof deferred<unknown>>}[]=[];
  let native:SourceBridge|undefined = {invoke:async <T>(command:string,args:Record<string,unknown>|Uint8Array) => { const pending=deferred<unknown>(); calls.push({command,args:args as Record<string,unknown>,pending}); return await pending.promise as T; }};
  let prepare:(profile:PlanarFaceProfile)=>Promise<ReturnType<typeof faceProfileGeometry>> = async profile=>faceProfileGeometry(profile);
  const controller=createFaceProfiles(state,{native:()=>native,context:()=>context,name:()=> 'part.3mf',prepare:profile=>prepare(profile),initialSelectionId});
  const latest=(command:string) => { const call=calls.findLast(c=>c.command===command); assert.ok(call,command); return call; };
  function response(call=latest('face_profile')):FaceProfileResponse { return {request_id:call.args.requestId as number,source_id:call.args.sourceId as number,selection_id:call.args.selectionId as number,profile:plane(call.args.faceId as number)}; }
  const ready=async () => { const work=controller.sync(); latest('set_face_profile_context').pending.resolve(undefined); await tick(); latest('face_profile').pending.resolve(response()); await work; };
  return {state,calls,controller,latest,response,ready,setContext:(value:FaceProfileContext|null)=>{context=value;},setNative:(value:SourceBridge|undefined)=>{native=value;},setPrepare:(value:typeof prepare)=>{prepare=value;}};
}
test('profile extraction awaits native context and exports only matching native identities',async()=>{
  const f=fixture(), work=f.controller.sync();
  assert.equal(f.state.pending,true); assert.equal(f.controller.canExport(),false); assert.equal(f.calls.length,1);
  assert.deepEqual(f.latest('set_face_profile_context').args,{requestId:1,sourceId:9,faceId:2,selectionId:1});
  f.latest('set_face_profile_context').pending.resolve(undefined); await tick();
  assert.deepEqual(f.latest('face_profile').args,f.latest('set_face_profile_context').args);
  f.latest('face_profile').pending.resolve(f.response()); await work;
  assert.equal(f.controller.canExport(),true);
  const save=f.controller.exportDxf(); assert.equal(f.state.exporting,true); assert.equal(f.controller.canExport(),false);
  assert.deepEqual(f.latest('export_face_dxf').args,{requestId:1,sourceId:9,faceId:2,selectionId:1,suggestedName:'part-face-3.dxf'});
  f.latest('export_face_dxf').pending.resolve('/tmp/part.dxf'); await save;
  assert.equal(f.state.note,'saved'); assert.equal(f.controller.canExport(),true);
});
test('no selection, repeated context and unavailable bridge do not invoke geometry or export',async()=>{
  const f=fixture(); f.setContext(null); await f.controller.sync(); await f.controller.exportDxf(); assert.equal(f.calls.length,0);
  f.setContext({requestId:1,sourceId:9,faceId:2}); await f.ready(); const length=f.calls.length; await f.controller.sync(); assert.equal(f.calls.length,length);
  f.setNative(undefined); assert.equal(f.controller.canExport(),false); await f.controller.exportDxf(); assert.equal(f.calls.length,length);
});
test('clearing while context is being installed cannot start stale extraction',async()=>{
  const f=fixture(), work=f.controller.sync(), install=f.latest('set_face_profile_context');
  f.setContext(null); f.controller.clear();
  assert.deepEqual(f.latest('set_face_profile_context').args,{requestId:1,sourceId:9,selectionId:2,faceId:null});
  install.pending.resolve(undefined); await work;
  assert.equal(f.calls.filter(c=>c.command==='face_profile').length,0); assert.equal(f.state.pending,false); assert.equal(f.state.response,null);
});
for (const change of ['selection','source','result','source-view'] as const) test(`${change} change clears profile before native reply and ignores obsolete success`,async()=>{
  const f=fixture(), first=f.controller.sync(); f.latest('set_face_profile_context').pending.resolve(undefined); await tick(); const old=f.latest('face_profile');
  f.setContext(change==='source-view'?null:{requestId:change==='result'?2:1,sourceId:change==='source'?10:9,faceId:change==='selection'?3:2});
  const newer=f.controller.sync(); assert.equal(f.state.response,null); assert.equal(f.controller.canExport(),false);
  old.pending.resolve(f.response(old)); await first; assert.equal(f.state.response,null);
  if(change!=='source-view'){ f.latest('set_face_profile_context').pending.resolve(undefined); await tick(); f.latest('face_profile').pending.resolve(f.response()); }
  await newer;
});
test('selection A to B to A never revives the first A response',async()=>{
  const f=fixture(), first=f.controller.sync(); f.latest('set_face_profile_context').pending.resolve(undefined); await tick(); const old=f.latest('face_profile');
  f.setContext({requestId:1,sourceId:9,faceId:3}); const middle=f.controller.sync(), middleInstall=f.latest('set_face_profile_context');
  f.setContext({requestId:1,sourceId:9,faceId:2}); const last=f.controller.sync();
  old.pending.resolve(f.response(old)); middleInstall.pending.resolve(undefined); await Promise.all([first,middle]); assert.equal(f.state.response,null);
  f.latest('set_face_profile_context').pending.resolve(undefined); await tick(); f.latest('face_profile').pending.resolve(f.response()); await last;
  assert.equal(f.state.response?.selection_id,3);
});
test('obsolete native refusal cannot overwrite a newer successful profile',async()=>{
  const f=fixture(), oldWork=f.controller.sync(); f.latest('set_face_profile_context').pending.resolve(undefined); await tick(); const old=f.latest('face_profile');
  f.setContext({requestId:1,sourceId:9,faceId:3}); await f.ready(); const accepted=f.state.response;
  old.pending.reject({code:'dxf.unsupported_curve'}); await oldWork; assert.equal(f.state.response,accepted); assert.equal(f.state.error,null);
});
for (const field of ['request_id','source_id','selection_id','face_id'] as const) test(`response with stale ${field} is refused`,async()=>{
  const f=fixture(), work=f.controller.sync(); f.latest('set_face_profile_context').pending.resolve(undefined); await tick(); const response=f.response();
  if(field==='face_id')response.profile.face_id++; else response[field]++;
  f.latest('face_profile').pending.resolve(response); await work; assert.equal(f.state.response,null); assert.equal(f.controller.canExport(),false); assert.equal((f.state.error as {code:string}).code,'engine.stale');
});
for(const command of ['set_face_profile_context','face_profile']) test(`${command} refusal is contextual and keeps STEP state outside the controller`,async()=>{
  const f=fixture(), work=f.controller.sync(), code='dxf.unsupported_curve';
  if(command==='face_profile'){f.latest('set_face_profile_context').pending.resolve(undefined);await tick();}
  f.latest(command).pending.reject({code}); await work;
  assert.equal((f.state.error as {code:string}).code,code); assert.equal(f.state.pending,false); assert.equal(f.state.response,null); assert.equal(f.controller.canExport(),false);
  assert.ok(f.calls.every(call=>!['invalidate_result','convert_document'].includes(call.command)));
});
test('malformed native curve is never displayed or exportable',async()=>{
  const f=fixture(), work=f.controller.sync(); f.latest('set_face_profile_context').pending.resolve(undefined); await tick(); const bad=f.response(); bad.profile.loops[0].segments=[{kind:'circle',center_mm:[0,0],radius_mm:NaN}];
  f.latest('face_profile').pending.resolve(bad); await work; assert.equal(f.state.response,null); assert.equal((f.state.error as {code:string}).code,'dxf.invalid_profile');
});
test('duplicate export is blocked; cancellation and IO failure preserve current profile',async()=>{
  const f=fixture();await f.ready(); const accepted=f.state.response, save=f.controller.exportDxf(); await f.controller.exportDxf(); assert.equal(f.calls.filter(c=>c.command==='export_face_dxf').length,1);
  f.latest('export_face_dxf').pending.resolve(null); await save; assert.equal(f.state.note,'cancelled'); assert.equal(f.state.response,accepted);
  const retry=f.controller.exportDxf(); f.latest('export_face_dxf').pending.reject({code:'io.file'}); await retry; assert.equal(f.state.note,null); assert.equal(f.state.response,accepted); assert.equal(f.controller.canExport(),true);
});
test('selection change during save invalidates native context and ignores saved/error replies',async()=>{
  for(const reject of [false,true]){
    const f=fixture();await f.ready();const save=f.controller.exportDxf(), old=f.latest('export_face_dxf'); f.setContext(null); f.controller.clear();
    assert.equal(f.latest('set_face_profile_context').args.faceId,null); assert.equal(f.controller.canExport(),false);
    if(reject)old.pending.reject({code:'engine.stale'});else old.pending.resolve('/tmp/stale.dxf'); await save;
    assert.equal(f.state.response,null);assert.equal(f.state.note,null);assert.equal(f.state.error,null);assert.equal(f.state.exporting,false);
  }
});
test('current native stale export removes profile but preserves contextual refusal',async()=>{
  const f=fixture();await f.ready();const save=f.controller.exportDxf();f.latest('export_face_dxf').pending.reject({code:'engine.stale'});await save;
  assert.equal(f.state.response,null);assert.equal(f.controller.canExport(),false);assert.equal((f.state.error as {code:string}).code,'engine.stale');
});
test('dispose invalidates native context and rejects every late publication',async()=>{
  const f=fixture(), work=f.controller.sync(); f.latest('set_face_profile_context').pending.resolve(undefined);await tick();const old=f.latest('face_profile');f.controller.dispose();
  assert.equal(f.latest('set_face_profile_context').args.faceId,null);old.pending.resolve(f.response(old));await work;await f.controller.sync();await f.controller.exportDxf();assert.equal(f.state.response,null);assert.equal(f.controller.canExport(),false);
});
test('native profile cannot publish or export before its display worker completes',async()=>{
  const f=fixture(), prepared=deferred<ReturnType<typeof faceProfileGeometry>>(); f.setPrepare(()=>prepared.promise);
  const work=f.controller.sync();f.latest('set_face_profile_context').pending.resolve(undefined);await tick();f.latest('face_profile').pending.resolve(f.response());await tick();
  assert.equal(f.state.response,null);assert.equal(f.state.geometry,null);assert.equal(f.state.pending,true);assert.equal(f.controller.canExport(),false);
  prepared.resolve(faceProfileGeometry(plane()));await work;assert.equal(f.state.pending,false);assert.ok(f.state.geometry);assert.equal(f.controller.canExport(),true);
});
test('late display worker success or error cannot overwrite a newer face',async()=>{
  for(const reject of [false,true]){
    const f=fixture(), prepared=deferred<ReturnType<typeof faceProfileGeometry>>();f.setPrepare(()=>prepared.promise);
    const old=f.controller.sync();f.latest('set_face_profile_context').pending.resolve(undefined);await tick();f.latest('face_profile').pending.resolve(f.response());await tick();
    f.setPrepare(async profile=>faceProfileGeometry(profile));f.setContext({requestId:1,sourceId:9,faceId:3});await f.ready();const accepted=f.state.response, geometry=f.state.geometry;
    if(reject)prepared.reject({code:'dxf.invalid_profile'});else prepared.resolve(faceProfileGeometry(plane()));await old;
    assert.equal(f.state.response,accepted);assert.equal(f.state.geometry,geometry);assert.equal(f.state.error,null);assert.equal(f.controller.canExport(),true);
  }
});
test('live context guard blocks export before the invalidation effect runs',async()=>{
  const f=fixture();await f.ready();f.setContext({requestId:2,sourceId:9,faceId:2});assert.equal(f.controller.canExport(),false);await f.controller.exportDxf();assert.equal(f.calls.filter(c=>c.command==='export_face_dxf').length,0);
});
test('newly mounted UI can seed selection epochs above a previous native session',async()=>{
  const f=fixture(1_800_000_000_000_000);await f.ready();assert.equal(f.state.response?.selection_id,1_800_000_000_000_001);assert.equal(f.controller.canExport(),true);
});
