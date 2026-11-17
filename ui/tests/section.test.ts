import {test} from 'node:test';
import {strict as assert} from 'node:assert';
import {createSections,type SectionContext,type SectionState} from '../src/lib/section.ts';
import {sectionProfileGeometry} from '../src/lib/face-profile-geometry.ts';
import type {SourceBridge} from '../src/lib/source.ts';
import type {ProfileFrame,SectionContextResponse,SectionProfile,SectionResponse} from '../src/lib/types.ts';
function deferred<T>(){let resolve!:(value:T)=>void,reject!:(error:unknown)=>void;const promise=new Promise<T>((yes,no)=>{resolve=yes;reject=no;});return {resolve,reject,promise};}
const tick=async()=>{for(let i=0;i<15;i++)await Promise.resolve();};
const frame=(z=2):ProfileFrame=>({origin_mm:[0,0,z],x_axis:[1,0,0],y_axis:[0,1,0],normal:[0,0,1]});
const profile=(solidIds=[0,2],empty=false):SectionProfile=>({frame:frame(),bodies:solidIds.map(solid_id=>({solid_id,loops:empty?[]:[{component_id:0,is_outer:true,segments:[{kind:'circle',center_mm:[solid_id*10,0],radius_mm:2}]}]}))});
function fixture(){
  const state:SectionState={response:null,frame:null,geometry:null,pending:false,exporting:false,error:null,note:null};
  let context:SectionContext|null={requestId:1,sourceId:5,input:{basis:'xy',face_id:null,offset_mm:2,solid_ids:[0,2]}};
  const calls:{command:string;args:Record<string,any>;pending:ReturnType<typeof deferred<unknown>>}[]=[];
  let native:SourceBridge|undefined={invoke:async <T>(command:string,args:Record<string,unknown>|Uint8Array)=>{const pending=deferred<unknown>();calls.push({command,args:args as Record<string,unknown>,pending});return await pending.promise as T;}};
  const timers=new Map<number,()=>void>();let serial=0,clears=0;
  let prepare:(profile:SectionProfile)=>Promise<ReturnType<typeof sectionProfileGeometry>>=async profile=>sectionProfileGeometry(profile);
  const controller=createSections(state,{native:()=>native,context:()=>context,name:()=> 'part.stl',prepare:profile=>prepare(profile),clearPrepared:()=>{clears++;},schedule:(callback,delay)=>{assert.equal(delay,150);const id=++serial;timers.set(id,callback);return id;},clearSchedule:handle=>{timers.delete(handle as number);}});
  const latest=(command:string)=>{const call=calls.findLast(call=>call.command===command);assert.ok(call,command);return call;};
  const ack=(call=latest('set_section_context')):SectionContextResponse=>({request_id:call.args.requestId,source_id:call.args.sourceId,section_id:call.args.sectionId,target:'step',preparation_id:null,frame:frame(call.args.input?.offset_mm)});
  const response=(call=latest('section_profile'),section=profile()):SectionResponse=>({request_id:call.args.requestId,source_id:call.args.sourceId,section_id:call.args.sectionId,target:'step',preparation_id:null,profile:section});
  const flush=()=>{const callbacks=[...timers.values()];timers.clear();callbacks.forEach(callback=>callback());};
  const installed=async()=>{flush();await tick();latest('set_section_context').pending.resolve(ack());await tick();};
  const ready=async(section=profile())=>{const work=controller.sync();await installed();flush();await tick();latest('section_profile').pending.resolve(response(undefined,section));await work;};
  return {controller,state,calls,timers,latest,ack,response,flush,installed,ready,clears:()=>clears,setContext:(value:SectionContext|null)=>{context=value;},context:()=>context,setNative:(value:SourceBridge|undefined)=>{native=value;},setPrepare:(value:typeof prepare)=>{prepare=value;}};
}
test('section invalidates immediately then coalesces native frame/extraction and exports by identity only',async()=>{
  const f=fixture(),work=f.controller.sync();assert.equal(f.state.pending,true);assert.equal(f.state.frame,null);assert.equal(f.controller.canExport(),false);
  assert.deepEqual(f.latest('set_section_context').args,{requestId:1,sourceId:5,sectionId:1,input:null});assert.equal(f.timers.size,1);assert.equal(f.calls.filter(call=>call.command==='section_profile').length,0);
  await f.installed();assert.deepEqual(f.state.frame,frame());assert.deepEqual(f.latest('set_section_context').args,{requestId:1,sourceId:5,sectionId:2,input:{basis:'xy',face_id:null,offset_mm:2,solid_ids:[0,2]}});
  assert.deepEqual(f.latest('section_profile').args,{requestId:1,sourceId:5,sectionId:2});f.latest('section_profile').pending.resolve(f.response());await work;assert.equal(f.controller.canExport(),true);
  const save=f.controller.exportDxf();assert.equal(f.state.exporting,true);assert.deepEqual(f.latest('export_section_dxf').args,{requestId:1,sourceId:5,sectionId:2,suggestedName:'part-section.dxf'});f.latest('export_section_dxf').pending.resolve('/tmp/part.dxf');await save;assert.equal(f.state.note,'saved');
  assert.ok(f.calls.every(call=>!['invalidate_result','convert_document','export_face_dxf','set_face_profile_context'].includes(call.command)));
});
test('inactive or repeated context causes no unnecessary IPC and unavailable bridge blocks export',async()=>{
  const f=fixture();f.setContext(null);await f.controller.sync();await f.controller.exportDxf();assert.equal(f.calls.length,0);f.setContext({requestId:1,sourceId:5,input:{basis:'xy',face_id:null,offset_mm:2,solid_ids:[0,2]}});await f.ready();const count=f.calls.length;await f.controller.sync();assert.equal(f.calls.length,count);f.setNative(undefined);assert.equal(f.controller.canExport(),false);await f.controller.exportDxf();assert.equal(f.calls.length,count);
});
test('numeric scrubbing bounds native face-frame resolution to one active and one latest input',async()=>{
  const f=fixture(),first=f.controller.sync();f.flush();await tick();const old=f.latest('set_section_context');
  f.setContext({...f.context()!,input:{...f.context()!.input,offset_mm:3}});const second=f.controller.sync();assert.equal(f.state.frame,null);assert.equal(f.latest('set_section_context').args.input,null);f.flush();await tick();
  f.setContext({...f.context()!,input:{...f.context()!.input,offset_mm:-4}});const last=f.controller.sync();await second;f.flush();await tick();assert.equal(f.calls.filter(call=>call.command==='set_section_context'&&call.args.input).length,1);
  old.pending.resolve(f.ack(old));await first;await tick();assert.equal(f.calls.filter(call=>call.command==='set_section_context'&&call.args.input).length,2);assert.equal(f.latest('set_section_context').args.input.offset_mm,-4);
  await f.installed();assert.equal(f.calls.filter(call=>call.command==='section_profile').length,1);assert.equal(f.state.frame?.origin_mm[2],-4);f.latest('section_profile').pending.resolve(f.response());await last;
});
for(const change of ['plane','basis','bodies','source','result','tool-close'] as const)test(`${change} change rejects obsolete section success`,async()=>{
  const f=fixture(),first=f.controller.sync();await f.installed();f.flush();await tick();const old=f.latest('section_profile');
  const next=structuredClone(f.context()!);if(change==='plane')next.input.offset_mm=4;if(change==='basis'){next.input.basis='xz';}if(change==='bodies')next.input.solid_ids=[2];if(change==='source')next.sourceId++;if(change==='result')next.requestId++;
  f.setContext(change==='tool-close'?null:next);const second=f.controller.sync();assert.equal(f.state.response,null);assert.equal(f.controller.canExport(),false);old.pending.resolve(f.response(old));await first;assert.equal(f.state.response,null);
  if(change!=='tool-close'){await f.installed();f.flush();await tick();f.latest('section_profile').pending.resolve(f.response(undefined,profile(next.input.solid_ids)));}await second;
});
test('A to B to A never revives the first native section',async()=>{
  const f=fixture(),first=f.controller.sync();await f.installed();f.flush();await tick();const old=f.latest('section_profile'),original=structuredClone(f.context()!);
  f.setContext({...original,input:{...original.input,offset_mm:8}});const middle=f.controller.sync();f.setContext(original);const last=f.controller.sync();old.pending.resolve(f.response(old));await Promise.all([first,middle]);assert.equal(f.state.response,null);await f.installed();f.latest('section_profile').pending.resolve(f.response());await last;assert.equal(f.state.response?.section_id,6);
});
for(const command of ['set_section_context','section_profile'])for(const field of ['request_id','source_id','section_id'])test(`${command} rejects mismatched ${field}`,async()=>{
  const f=fixture(),work=f.controller.sync();let value:any;
  if(command==='section_profile'){await f.installed();value=f.response();}else {f.flush();await tick();value=f.ack();}value[field]++;f.latest(command).pending.resolve(value);await work;assert.equal((f.state.error as {code:string}).code,'engine.stale');assert.equal(f.state.response,null);assert.equal(f.controller.canExport(),false);
});
for(const code of ['dxf.section_coplanar','dxf.section_tangent','dxf.section_open','dxf.unsupported_curve','dxf.too_complex'])test(`${code} keeps the native plane visible and never invalidates the STEP`,async()=>{
  const f=fixture(),work=f.controller.sync();await f.installed();f.flush();await tick();f.latest('section_profile').pending.reject({code});await work;assert.deepEqual(f.state.frame,frame());assert.equal((f.state.error as {code:string}).code,code);assert.equal(f.state.pending,false);assert.equal(f.state.response,null);assert.equal(f.controller.canExport(),false);assert.ok(f.calls.every(call=>!['invalidate_result','convert_document'].includes(call.command)));
});
test('empty selected bodies stay explicit; all-empty section cannot export',async()=>{
  const f=fixture();await f.ready(profile([0,2],true));assert.equal(f.state.response?.profile.bodies.length,2);assert.equal(f.state.geometry,null);assert.equal(f.controller.canExport(),false);await f.controller.exportDxf();assert.equal(f.calls.filter(call=>call.command==='export_section_dxf').length,0);
  const next={...f.context()!,input:{...f.context()!.input,offset_mm:3}};f.setContext(next);const mixed=profile();mixed.bodies[0].loops=[];await f.ready(mixed);assert.equal(f.state.response?.profile.bodies[0].loops.length,0);assert.equal(f.controller.canExport(),true);
});
test('missing or reordered selected solid identities refuse a partial section',async()=>{
  for(const ids of [[0],[2,0]]){const f=fixture(),work=f.controller.sync();await f.installed();f.flush();await tick();f.latest('section_profile').pending.resolve(f.response(undefined,profile(ids)));await work;assert.equal(f.state.response,null);assert.equal((f.state.error as {code:string}).code,'engine.stale');}
});
test('late geometry publication or refusal cannot alter a newer section',async()=>{
  for(const reject of [false,true]){const f=fixture(),prepared=deferred<ReturnType<typeof sectionProfileGeometry>>();f.setPrepare(()=>prepared.promise);const first=f.controller.sync();await f.installed();f.flush();await tick();f.latest('section_profile').pending.resolve(f.response());await tick();assert.equal(f.state.response,null);assert.equal(f.controller.canExport(),false);
    f.setPrepare(async profile=>sectionProfileGeometry(profile));f.setContext({...f.context()!,input:{...f.context()!.input,offset_mm:8}});const next=f.controller.sync();f.flush();if(reject)prepared.reject({code:'dxf.invalid_profile'});else prepared.resolve(sectionProfileGeometry(profile()));await first;await f.installed();f.latest('section_profile').pending.resolve(f.response());await next;assert.equal(f.state.response?.section_id,4);assert.equal(f.state.error,null);}
});
test('changing the plane during save cancels native context and ignores late success or failure',async()=>{
  for(const reject of [false,true]){const f=fixture();await f.ready();const save=f.controller.exportDxf(),old=f.latest('export_section_dxf');f.setContext(null);f.controller.clear();assert.equal(f.latest('set_section_context').args.input,null);assert.equal(f.controller.canExport(),false);if(reject)old.pending.reject({code:'engine.stale'});else old.pending.resolve('/tmp/old.dxf');await save;assert.equal(f.state.note,null);assert.equal(f.state.error,null);assert.equal(f.state.frame,null);}
});
test('save cancellation and IO errors preserve section; native stale removes its export',async()=>{
  const f=fixture();await f.ready();let save=f.controller.exportDxf();await f.controller.exportDxf();assert.equal(f.calls.filter(call=>call.command==='export_section_dxf').length,1);f.latest('export_section_dxf').pending.resolve(null);await save;assert.equal(f.state.note,'cancelled');const accepted=f.state.response;
  save=f.controller.exportDxf();f.latest('export_section_dxf').pending.reject({code:'io.file'});await save;assert.equal(f.state.response,accepted);assert.equal(f.controller.canExport(),true);
  save=f.controller.exportDxf();f.latest('export_section_dxf').pending.reject({code:'engine.stale'});await save;assert.equal(f.state.response,null);assert.equal(f.controller.canExport(),false);
});
test('live input guard blocks export before a reactive invalidation and dispose cancels timers',async()=>{
  const f=fixture();await f.ready();f.setContext({...f.context()!,input:{...f.context()!.input,solid_ids:[0]}});assert.equal(f.controller.canExport(),false);await f.controller.exportDxf();assert.equal(f.calls.filter(call=>call.command==='export_section_dxf').length,0);
  const pending=f.controller.sync();assert.equal(f.timers.size,1);f.controller.dispose();await pending;assert.equal(f.timers.size,0);assert.equal(f.latest('set_section_context').args.input,null);assert.equal(f.state.response,null);await f.controller.sync();assert.equal(f.controller.canExport(),false);
});
test('long native section keeps a single latest pending input instead of queuing intermediate OCCT jobs',async()=>{
  const f=fixture(),first=f.controller.sync();await f.installed();const old=f.latest('section_profile'),pending:Promise<void>[]=[];
  for(let offset=3;offset<=10;offset++){
    f.setContext({...f.context()!,input:{...f.context()!.input,offset_mm:offset}});pending.push(f.controller.sync());assert.equal(f.latest('set_section_context').args.input,null);f.flush();await tick();
    assert.equal(f.calls.filter(call=>call.command==='set_section_context'&&call.args.input).length,1);assert.equal(f.calls.filter(call=>call.command==='section_profile').length,1);
  }
  old.pending.resolve(f.response(old));await first;await tick();assert.equal(f.calls.filter(call=>call.command==='set_section_context'&&call.args.input).length,2);assert.equal(f.latest('set_section_context').args.input.offset_mm,10);
  await f.installed();f.latest('section_profile').pending.resolve(f.response());await Promise.all(pending);assert.equal(f.calls.filter(call=>call.command==='section_profile').length,2);assert.equal(f.state.response?.section_id,18);
});
test('custom rotations keep the bounded queue and a late native frame cannot replace a newer drag result',async()=>{
  const f=fixture(),first=f.controller.sync();await f.installed();const old=f.latest('section_profile');
  const custom=(angle:number)=>({...f.context()!,input:{basis:'custom' as const,face_id:null,offset_mm:0,solid_ids:[0,2],custom_frame:{origin_mm:[10,10,2] as [number,number,number],x_axis:[1,0,0] as [number,number,number],y_axis:[0,Math.cos(angle),Math.sin(angle)] as [number,number,number],normal:[0,-Math.sin(angle),Math.cos(angle)] as [number,number,number]}}});
  f.setContext(custom(0.2));const middle=f.controller.sync();f.flush();await tick();f.setContext(custom(0.4));const latest=f.controller.sync();f.flush();await middle;
  assert.equal(f.calls.filter(call=>call.command==='set_section_context'&&call.args.input).length,1);old.pending.resolve(f.response(old));await first;await tick();const install=f.latest('set_section_context');assert.equal(install.args.input.basis,'custom');assert.equal(install.args.input.offset_mm,0);assert.ok(Math.abs(install.args.input.custom_frame.normal[1]+Math.sin(0.4))<1e-12);
  const ack=f.ack(install);ack.frame=install.args.input.custom_frame;install.pending.resolve(ack);await tick();const section=f.response();section.profile.frame=install.args.input.custom_frame;f.latest('section_profile').pending.resolve(section);await latest;assert.equal(f.state.response?.profile.frame.normal[1],-Math.sin(0.4));assert.equal(f.controller.canExport(),true);
});
test('drag invalidation starts no native work while held and destroying a queued rotation cannot resume it',async()=>{
  const f=fixture();await f.ready();const count=f.calls.filter(call=>call.command==='set_section_context'&&call.args.input).length;
  f.setContext(null);f.controller.clear();await f.controller.sync();f.flush();await tick();assert.equal(f.calls.filter(call=>call.command==='set_section_context'&&call.args.input).length,count);assert.equal(f.controller.canExport(),false);
  f.setContext({requestId:1,sourceId:5,input:{basis:'custom',face_id:null,offset_mm:0,solid_ids:[0,2],custom_frame:frame(7)}});const pending=f.controller.sync();assert.equal(f.timers.size,1);f.controller.dispose();f.flush();await pending;assert.equal(f.timers.size,0);assert.equal(f.calls.filter(call=>call.command==='set_section_context'&&call.args.input).length,count);assert.equal(f.state.response,null);
});
