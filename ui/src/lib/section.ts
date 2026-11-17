import type { SourceBridge } from './source.ts';
import type { ProfileFrame, SectionContextResponse, SectionInput, SectionProfile, SectionResponse } from './types.ts';
import type { FaceProfileGeometry } from './face-profile-geometry.ts';
export type SectionContext = {requestId:number;sourceId:number;input:SectionInput};
export type SectionState = {response:SectionResponse|null;frame:ProfileFrame|null;geometry:FaceProfileGeometry|null;pending:boolean;exporting:boolean;error:unknown|null;note:'saved'|'cancelled'|null};
type Dependencies = {native:()=>SourceBridge|undefined;context:()=>SectionContext|null;name:()=>string;prepare:(profile:SectionProfile)=>Promise<FaceProfileGeometry|null>;clearPrepared:()=>void;schedule:(callback:()=>void,delay:number)=>unknown;clearSchedule:(handle:unknown)=>void;initialSectionId?:number;onFrame?:(frame:ProfileFrame,input:SectionInput)=>void};
const key = (context:SectionContext|null) => context ? JSON.stringify(context) : '';
const stale = () => ({code:'engine.stale',message:'Stale STEP section.'});
/** Section context is read-only and independent of the face-profile and STEP export caches. */
export function createSections(state:SectionState,deps:Dependencies) {
  let context:SectionContext|null = null, sectionId = deps.initialSectionId ?? 0, disposed = false;
  type Job={id:number;context:SectionContext;bridge:SourceBridge;ready:boolean;resolve:()=>void};
  let timer:unknown,active:Job|null=null,queued:Job|null=null;
  const current = (id:number,captured:SectionContext) => !disposed && id === sectionId && key(context) === key(captured) && key(deps.context()) === key(captured);
  function stopTimer() {if(timer!==undefined)deps.clearSchedule(timer);timer=undefined;}
  function reset() { stopTimer(); deps.clearPrepared(); state.response = null; state.frame = null; state.geometry = null; state.pending = false; state.exporting = false; state.error = null; state.note = null; }
  function invalidateNative(captured:SectionContext,id:number){const bridge=deps.native();if(bridge)void bridge.invoke<SectionContextResponse>('set_section_context',{requestId:captured.requestId,sourceId:captured.sourceId,sectionId:id,input:null}).catch(()=>{});}
  function clear() {
    const previous = context; context = null; sectionId+=2; reset();queued?.resolve();queued=null;
    if (previous)invalidateNative(previous,sectionId);
  }
  function pump(){if(disposed||active||!queued?.ready)return;active=queued;queued=null;void run(active);}
  function sync():Promise<void> {
    if (disposed) return Promise.resolve();
    const next = deps.context();
    if (key(next) === key(context)) return Promise.resolve();
    if (!next) { clear(); return Promise.resolve(); }
    context = {...next,input:{...next.input,solid_ids:[...next.input.solid_ids],...(next.input.custom_frame?{custom_frame:{origin_mm:[...next.input.custom_frame.origin_mm],x_axis:[...next.input.custom_frame.x_axis],y_axis:[...next.input.custom_frame.y_axis],normal:[...next.input.custom_frame.normal]}}:{})}};
    const captured = context,id = sectionId+=2; reset();state.pending=true;queued?.resolve();queued=null;
    // The cheap invalidation is independent of a native call still finishing.
    // Reserve the next epoch for the actual input once the serial slot opens.
    invalidateNative(captured,id-1);
    const bridge = deps.native();
    if (!bridge) { state.pending = false; return Promise.resolve(); }
    return new Promise(resolve=>{
      const job:Job={id,context:captured,bridge,ready:false,resolve};queued=job;
      timer=deps.schedule(()=>{timer=undefined;if(queued===job){job.ready=true;pump();}},150);
    });
  }
  async function run(job:Job){
    const {id,context:captured,bridge}=job;
    const args = {requestId:captured.requestId,sourceId:captured.sourceId,sectionId:id};
    try {
      const installed = await bridge.invoke<SectionContextResponse>('set_section_context',{...args,input:captured.input});
      if (!current(id,captured)) return;
      if (installed.request_id !== captured.requestId || installed.source_id !== captured.sourceId || installed.section_id !== id) throw stale();
      if(installed.target!==(captured.input.target??'step')||installed.preparation_id!==(captured.input.target==='source'?captured.input.preparation_id:null))throw stale();
      if (!installed.frame || Object.values(installed.frame).some(axis=>axis.length !== 3 || axis.some(value=>!Number.isFinite(value)))) throw {code:'dxf.section_invalid_plane',message:'Invalid section frame.'};
      state.frame = installed.frame;
      deps.onFrame?.(installed.frame,captured.input);
      const response = await bridge.invoke<SectionResponse>('section_profile',args);
      if (!current(id,captured)) return;
      if (response.request_id !== captured.requestId || response.source_id !== captured.sourceId || response.section_id !== id || response.profile.bodies.length !== captured.input.solid_ids.length || response.profile.bodies.some((body,index)=>body.solid_id !== captured.input.solid_ids[index])) throw stale();
      if(response.target!==(captured.input.target??'step')||response.preparation_id!==(captured.input.target==='source'?captured.input.preparation_id:null))throw stale();
      if(captured.input.target==='source'&&response.profile.bodies.some(body=>body.loops.some(loop=>loop.segments.some(segment=>segment.kind!=='line'))))throw {code:'dxf.invalid_profile',message:'Source mesh section must contain only polygonal lines.'};
      const geometry = await deps.prepare(response.profile);
      if (!current(id,captured)) return;
      state.geometry = geometry; state.response = response;
    } catch (error) { if (current(id,captured)) state.error = error; }
    finally { if (current(id,captured)) state.pending = false;job.resolve();active=null;pump(); }
  }
  function canExport() {return !disposed && !!deps.native() && !!context && current(sectionId,context) && !state.pending && !state.exporting && !!state.response && !!state.geometry;}
  async function exportDxf() {
    if (!canExport()) return;
    const captured = context!,id = sectionId; state.exporting = true; state.error = null; state.note = null;
    try {
      const path = await deps.native()!.invoke<string|null>('export_section_dxf',{requestId:captured.requestId,sourceId:captured.sourceId,sectionId:id,suggestedName:`${deps.name().replace(/\.(stl|3mf)$/i,'')}-${captured.input.target==='source'?'mesh-section':'section'}.dxf`});
      if (current(id,captured)) state.note = path ? 'saved' : 'cancelled';
    } catch(error) {if (current(id,captured)) {state.error = error;if ((error as {code?:string})?.code === 'engine.stale') {state.response = null;state.geometry = null;}}}
    finally {if (current(id,captured)) state.exporting = false;}
  }
  function dispose() {clear();disposed = true;}
  return {sync,clear,canExport,exportDxf,dispose};
}
