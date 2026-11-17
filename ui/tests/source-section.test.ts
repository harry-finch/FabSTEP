import {test} from 'node:test';
import {strict as assert} from 'node:assert';
import {createSourceSectionPreparation,initialSourceSectionInput,type SourceSectionPreparationContext,type SourceSectionPreparationState} from '../src/lib/source-section.ts';
import {createSections,type SectionContext,type SectionState} from '../src/lib/section.ts';
import {createWorkflow,type WorkflowState} from '../src/lib/workflow.ts';
import {sectionProfileGeometry} from '../src/lib/face-profile-geometry.ts';
import type {Options,SectionContextResponse,SectionInput,SectionProfile,SectionResponse,SourceSectionMetadata,SourceSectionPreparationResponse} from '../src/lib/types.ts';
import type {SourceBridge} from '../src/lib/source.ts';
const options:Options={input_unit:'mm',reconstruction:'surfaces',weld_tolerance_mm:1e-6,plane_tolerance_mm:1e-6,cylinder_fit_tolerance_mm:1e-5,max_deviation_mm:0.05};
const frame={origin_mm:[0,0,3] as [number,number,number],x_axis:[1,0,0] as [number,number,number],y_axis:[0,1,0] as [number,number,number],normal:[0,0,1] as [number,number,number]};
function metadata():SourceSectionMetadata{return {source:{format:'stl',unit:'mm',unit_origin:'selected'},settings:{...options},bounds_mm:[[10,20,2],[30,40,4]],bodies:[2,4].map(solid_id=>({solid_id,analysis_body_id:`stl/component/${solid_id}`,name:`Body ${solid_id}`,provenance:{generated_name:true},bounds_mm:[[10,20,2],[30,40,4]],triangles:12,input_triangles:12,welded_vertices:0,volume_mm3:800,diagnostics:[]})),diagnostics:[{code:'mesh.orientation_reversed',severity:'info',message:'Reversed',count:1}]};}
function polygon(ids=[2,4]):SectionProfile{return {frame,bodies:ids.map(solid_id=>({solid_id,loops:[{component_id:0,is_outer:true,segments:[{kind:'line',start_mm:[0,0],end_mm:[1,0]},{kind:'line',start_mm:[1,0],end_mm:[1,1]},{kind:'line',start_mm:[1,1],end_mm:[0,1]},{kind:'line',start_mm:[0,1],end_mm:[0,0]}]}]}))};}
function deferred<T>(){let resolve!:(value:T)=>void,reject!:(error:unknown)=>void;const promise=new Promise<T>((yes,no)=>{resolve=yes;reject=no;});return {resolve,reject,promise};}
const tick=async()=>{for(let i=0;i<15;i++)await Promise.resolve();};
function fixture(){
  const state:SourceSectionPreparationState={response:null,pending:false,error:null};let context:SourceSectionPreparationContext|null={requestId:1,sourceId:9,options:{...options}};
  const calls:{command:string;args:Record<string,any>;pending:ReturnType<typeof deferred<unknown>>}[]=[],ready:SourceSectionPreparationResponse[]=[];
  const bridge:SourceBridge={invoke:async <T>(command:string,args:Record<string,unknown>|Uint8Array)=>{const pending=deferred<unknown>();calls.push({command,args:args as Record<string,unknown>,pending});return await pending.promise as T;}};
  const controller=createSourceSectionPreparation(state,{native:()=>bridge,context:()=>context,onReady:response=>ready.push(response)});
  const latest=()=>{const call=calls.findLast(call=>call.args.options);assert.ok(call);return call;};
  const response=(call=latest()):SourceSectionPreparationResponse=>({request_id:call.args.requestId,source_id:call.args.sourceId,preparation_id:call.args.preparationId,metadata:metadata()});
  return {state,context:()=>context,setContext:(value:SourceSectionPreparationContext|null)=>{context=value;},calls,ready,controller,latest,response};
}
test('preparation publishes authoritative source IDs, names, bounds and diagnostics without a STEP result',async()=>{
  const f=fixture(),work=f.controller.sync();assert.equal(f.state.pending,true);assert.equal(f.calls[0].args.options,null);assert.equal(f.latest().args.preparationId,2);assert.deepEqual(f.latest().args.options,options);
  const native=f.response();f.latest().pending.resolve(native);await work;assert.equal(f.state.response,native);assert.equal(f.ready.length,1);assert.equal(f.state.pending,false);
  assert.deepEqual(initialSourceSectionInput(native.metadata!,2),{target:'source',preparation_id:2,basis:'xy',face_id:null,offset_mm:3,solid_ids:[2,4]});assert.deepEqual(f.state.response?.metadata?.bounds_mm,[[10,20,2],[30,40,4]]);assert.equal(f.state.response?.metadata?.diagnostics[0].code,'mesh.orientation_reversed');assert.ok(f.calls.every(call=>call.command==='prepare_source_section'));
});
for(const change of ['source','options','unit','view','close'] as const)test(`${change} invalidates preparation and rejects a late metadata publication`,async()=>{
  const f=fixture(),first=f.controller.sync(),old=f.latest(),next=structuredClone(f.context()!);
  if(change==='source'){next.requestId++;next.sourceId++;}if(change==='options'){next.options.weld_tolerance_mm=0.001;}if(change==='unit')next.options.input_unit='inch';
  f.setContext(change==='view'||change==='close'?null:next);const last=f.controller.sync();assert.equal(f.state.response,null);old.pending.resolve(f.response(old));await first;assert.equal(f.ready.length,0);
  if(change!=='view'&&change!=='close'){f.latest().pending.resolve(f.response());}await last;
});
test('one active preparation and only the final queued options are validated',async()=>{
  const f=fixture(),first=f.controller.sync(),old=f.latest(),pending:Promise<void>[]=[];
  for(let i=2;i<=8;i++){f.setContext({...f.context()!,options:{...options,weld_tolerance_mm:i*1e-6}});pending.push(f.controller.sync());assert.equal(f.calls.filter(call=>call.args.options).length,1);}
  old.pending.resolve(f.response(old));await first;await tick();assert.equal(f.calls.filter(call=>call.args.options).length,2);assert.equal(f.latest().args.options.weld_tolerance_mm,8e-6);f.latest().pending.resolve(f.response());await Promise.all(pending);assert.equal(f.ready.length,1);
});
for(const field of ['request_id','source_id','preparation_id'])test(`mismatched preparation ${field} cannot publish`,async()=>{
  const f=fixture(),work=f.controller.sync(),response=f.response();response[field as 'request_id']++;f.latest().pending.resolve(response);await work;assert.equal(f.state.response,null);assert.equal((f.state.error as {code:string}).code,'engine.stale');
});
test('source validation failure stays local; disposing cancels queued and late preparation',async()=>{
  const f=fixture(),work=f.controller.sync();f.latest().pending.reject({code:'mesh.open',message:'Open mesh.'});await work;assert.equal((f.state.error as {code:string}).code,'mesh.open');assert.equal(f.state.response,null);
  f.setContext({...f.context()!,requestId:2});const pending=f.controller.sync(),old=f.latest();f.setContext({...f.context()!,requestId:3});const queued=f.controller.sync();f.controller.dispose();await queued;old.pending.resolve(f.response(old));await pending;assert.equal(f.ready.length,0);assert.equal(f.state.response,null);assert.equal(f.calls.at(-1)!.args.options,null);assert.equal(f.calls.filter(call=>call.args.options).length,2);
});
test('metadata with guessed, duplicate, oversized or invalid body identities/bounds is refused',async()=>{
  for(const mutate of [(m:SourceSectionMetadata)=>{m.bodies[1].solid_id=2;},(m:SourceSectionMetadata)=>{m.bodies[1].analysis_body_id=m.bodies[0].analysis_body_id;},(m:SourceSectionMetadata)=>{m.bounds_mm[1][2]=NaN;},(m:SourceSectionMetadata)=>{m.bodies=[];},(m:SourceSectionMetadata)=>{m.bodies[1].solid_id=33;}]){
    const f=fixture(),work=f.controller.sync(),response=f.response();mutate(response.metadata!);f.latest().pending.resolve(response);await work;assert.equal(f.state.response,null);assert.equal((f.state.error as {code:string}).code,'dxf.invalid_profile');}
});
function sourceSectionFixture(){
  const state:SectionState={response:null,frame:null,geometry:null,pending:false,exporting:false,error:null,note:null};let context:SectionContext|null={requestId:1,sourceId:9,input:initialSourceSectionInput(metadata(),20)};let serial=0;const timers=new Map<number,()=>void>();
  const calls:{command:string;args:Record<string,any>;pending:ReturnType<typeof deferred<unknown>>}[]=[];const inputs=new Map<number,SectionInput>();
  const bridge:SourceBridge={invoke:async <T>(command:string,args:Record<string,unknown>|Uint8Array)=>{const pending=deferred<unknown>(),values=args as Record<string,any>;calls.push({command,args:values,pending});if(command==='set_section_context'&&values.input)inputs.set(values.sectionId,values.input);return await pending.promise as T;}};
  const controller=createSections(state,{native:()=>bridge,context:()=>context,name:()=> 'mesh.stl',prepare:async profile=>sectionProfileGeometry(profile),clearPrepared:()=>{},schedule:callback=>{timers.set(++serial,callback);return serial;},clearSchedule:handle=>{timers.delete(handle as number);}});
  const latest=(command:string)=>{const call=calls.findLast(call=>call.command===command);assert.ok(call);return call;};
  const flush=async()=>{const callbacks=[...timers.values()];timers.clear();callbacks.forEach(callback=>callback());await tick();};
  const ack=():SectionContextResponse=>{const call=latest('set_section_context'),input=inputs.get(call.args.sectionId)!;return {request_id:call.args.requestId,source_id:call.args.sourceId,section_id:call.args.sectionId,target:input.target??'step',preparation_id:input.preparation_id??null,frame};};
  const response=():SectionResponse=>{const call=latest('section_profile'),input=inputs.get(call.args.sectionId)!;return {request_id:call.args.requestId,source_id:call.args.sourceId,section_id:call.args.sectionId,target:input.target??'step',preparation_id:input.preparation_id??null,profile:polygon()};};
  const ready=async()=>{const work=controller.sync();await flush();latest('set_section_context').pending.resolve(ack());await tick();latest('section_profile').pending.resolve(response());await work;};
  return {state,controller,calls,latest,flush,ack,response,ready,context:()=>context,setContext:(value:SectionContext|null)=>{context=value;}};
}
for(const command of ['set_section_context','section_profile'])for(const mismatch of ['target','preparation_id'])test(`source ${command} rejects a different ${mismatch}`,async()=>{
  const f=sourceSectionFixture(),work=f.controller.sync();await f.flush();if(command==='section_profile'){f.latest('set_section_context').pending.resolve(f.ack());await tick();}
  const value=command==='set_section_context'?f.ack():f.response();if(mismatch==='target')value.target='step';else value.preparation_id=21;f.latest(command).pending.resolve(value);await work;assert.equal(f.state.response,null);assert.equal(f.controller.canExport(),false);assert.equal((f.state.error as {code:string}).code,'engine.stale');
});
test('source profile refuses analytic curves rather than claiming reconstructed geometry',async()=>{
  const f=sourceSectionFixture(),work=f.controller.sync();await f.flush();f.latest('set_section_context').pending.resolve(f.ack());await tick();const response=f.response();response.profile.bodies[0].loops[0].segments=[{kind:'circle',center_mm:[0,0],radius_mm:1}];f.latest('section_profile').pending.resolve(response);await work;assert.equal(f.state.response,null);assert.equal((f.state.error as {code:string}).code,'dxf.invalid_profile');
});
test('switching target or preparation during an export cancels its publication',async()=>{
  for(const target of ['step','source'] as const){const f=sourceSectionFixture();await f.ready();const save=f.controller.exportDxf(),old=f.latest('export_section_dxf');f.setContext({...f.context()!,input:{...f.context()!.input,target,preparation_id:target==='source'?21:null}});const next=f.controller.sync();assert.equal(f.controller.canExport(),false);assert.equal(f.latest('set_section_context').args.input,null);old.pending.resolve('/tmp/stale.dxf');await save;assert.equal(f.state.note,null);f.setContext(null);f.controller.clear();await next;}
});
test('failed production STEP conversion still permits source preparation, section and export without enabling STEP export',async()=>{
  const calls:string[]=[],timers:(()=>void)[]=[],failure={code:'recognition.unsupported',message:'No analytic surface.'};
  const state:WorkflowState={options:{...options},bytes:null,name:'mesh.stl',format:'stl',current:1,fileToken:0,sourceId:null,uploadedSourceId:null,sourceRevision:0,result:null,preview:null,selectedId:'',errorState:null,noteKey:null,recognitionFailed:false,busy:false,queued:false,processing:null,topologyState:'unavailable',guidanceHints:[],guidedAttempt:null,fitSignal:0};
  const bridge:SourceBridge={invoke:async <T>(command:string,args:Record<string,unknown>|Uint8Array)=>{calls.push(command);const a=args as Record<string,any>;let value:unknown;
    if(command==='preview_document')value={request_id:a.requestId,source:{format:'stl',unit:'mm',unit_origin:'selected'},bodies:[],diagnostics:[]};
    if(command==='convert_document')throw failure;
    if(command==='prepare_source_section')value={request_id:a.requestId,source_id:a.sourceId,preparation_id:a.preparationId,metadata:a.options?metadata():null};
    if(command==='set_section_context')value={request_id:a.requestId,source_id:a.sourceId,section_id:a.sectionId,target:a.input?.target??'step',preparation_id:a.input?.preparation_id??null,frame:a.input?frame:null};
    if(command==='section_profile')value={request_id:a.requestId,source_id:a.sourceId,section_id:a.sectionId,target:'source',preparation_id:2,profile:polygon()};
    if(command==='export_section_dxf')value='/tmp/mesh-section.dxf';return value as T;}};
  const workflow=createWorkflow(state,{native:()=>bridge,serializableOptions:value=>({...value}),clearParameterSearch(){},resetSourceInspection(){},resetStepInspection(){},prepareSelection(){},interpretSourceZone(){},onResultReady(){},now:()=>0,schedule:callback=>{timers.push(callback);return callback;},clearSchedule:handle=>{const index=timers.indexOf(handle as ()=>void);if(index>=0)timers.splice(index,1);}});workflow.setSource(new Uint8Array([1,2,3]));timers.shift()!();await tick();
  assert.ok(state.preview);assert.equal(state.recognitionFailed,false);assert.equal(calls.includes('convert_document'),false);assert.equal(workflow.canReconstruct(),true);
  await workflow.reconstruct();
  assert.equal(state.result,null);assert.ok(state.preview);assert.equal(state.recognitionFailed,true);assert.equal(workflow.canExport(),false);const preview=state.preview,globalError=state.errorState;
  assert.deepEqual(globalError,{kind:'native',value:failure});
  const prepared:SourceSectionPreparationState={response:null,pending:false,error:null};let sectionInput:SectionInput|null=null;
  const prep=createSourceSectionPreparation(prepared,{native:()=>bridge,context:()=>({requestId:state.current,sourceId:state.sourceId!,options:state.options}),onReady:response=>{sectionInput=initialSourceSectionInput(response.metadata,response.preparation_id);}});await prep.sync();assert.ok(sectionInput);
  const sectionState:SectionState={response:null,frame:null,geometry:null,pending:false,exporting:false,error:null,note:null};const sections=createSections(sectionState,{native:()=>bridge,context:()=>({requestId:state.current,sourceId:state.sourceId!,input:sectionInput!}),name:()=>state.name,prepare:async profile=>sectionProfileGeometry(profile),clearPrepared(){},schedule:callback=>{timers.push(callback);return callback;},clearSchedule(){}});
  const job=sections.sync();timers.shift()!();await job;assert.equal(sections.canExport(),true);await sections.exportDxf();assert.equal(sectionState.note,'saved');assert.equal(workflow.canExport(),false);assert.equal(state.preview,preview);assert.equal(state.errorState,globalError);assert.equal(calls.filter(command=>command==='convert_document').length,1);assert.equal(calls.includes('export_step'),false);sections.dispose();prep.dispose();workflow.dispose();
});
