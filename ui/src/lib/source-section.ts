import type {Options,SectionInput,SourceSectionMetadata,SourceSectionPreparationResponse} from './types.ts';
import type {SourceBridge} from './source.ts';
export type SourceSectionPreparationContext={requestId:number;sourceId:number;options:Options};
export type SourceSectionPreparationState={response:SourceSectionPreparationResponse|null;pending:boolean;error:unknown|null};
type Dependencies={native:()=>SourceBridge|undefined;context:()=>SourceSectionPreparationContext|null;onReady:(response:SourceSectionPreparationResponse&{metadata:SourceSectionMetadata})=>void;initialPreparationId?:number};
const key=(context:SourceSectionPreparationContext|null)=>context?JSON.stringify(context):'';
const stale=()=>({code:'engine.stale',message:'Stale source section preparation.'});
function validMetadata(metadata:SourceSectionMetadata){
  const bounds=(value:number[][])=>value.length===2&&value.every(point=>point.length===3&&point.every(Number.isFinite))&&value[0].every((coordinate,index)=>coordinate<=value[1][index]);
  if(!bounds(metadata.bounds_mm)||!metadata.bodies.length||metadata.bodies.length>32)return false;
  const ids=new Set<string>();let previous=-1;
  for(const body of metadata.bodies){if(!Number.isInteger(body.solid_id)||body.solid_id<=previous||body.solid_id>=32||!body.analysis_body_id||ids.has(body.analysis_body_id)||!bounds(body.bounds_mm)||!Number.isInteger(body.triangles)||body.triangles<=0)return false;previous=body.solid_id;ids.add(body.analysis_body_id);}
  return true;
}
export function initialSourceSectionInput(metadata:SourceSectionMetadata,preparationId:number):SectionInput{return {target:'source',preparation_id:preparationId,basis:'xy',face_id:null,offset_mm:(metadata.bounds_mm[0][2]+metadata.bounds_mm[1][2])/2,solid_ids:metadata.bodies.map(body=>body.solid_id)};}
/** One validation in flight and one latest preparation; cancellation never touches STEP export. */
export function createSourceSectionPreparation(state:SourceSectionPreparationState,deps:Dependencies){
  type Job={id:number;context:SourceSectionPreparationContext;bridge:SourceBridge;resolve:()=>void};
  let context:SourceSectionPreparationContext|null=null,generation=deps.initialPreparationId??0,active:Job|null=null,queued:Job|null=null,disposed=false;
  const current=(job:Job)=>!disposed&&job.id===generation&&key(context)===key(job.context)&&key(deps.context())===key(job.context);
  function reset(){state.response=null;state.pending=false;state.error=null;}
  function invalidate(captured:SourceSectionPreparationContext,id:number){const bridge=deps.native();if(bridge)void bridge.invoke('prepare_source_section',{requestId:captured.requestId,sourceId:captured.sourceId,preparationId:id,options:null}).catch(()=>{});}
  function clear(){const previous=context;context=null;generation+=2;reset();queued?.resolve();queued=null;if(previous)invalidate(previous,generation);}
  function pump(){if(disposed||active||!queued)return;active=queued;queued=null;void run(active);}
  function sync():Promise<void>{
    if(disposed)return Promise.resolve();const next=deps.context();if(key(next)===key(context))return Promise.resolve();if(!next){clear();return Promise.resolve();}
    context={...next,options:{...next.options}};generation+=2;reset();state.pending=true;queued?.resolve();queued=null;invalidate(context,generation-1);
    const bridge=deps.native();if(!bridge){state.pending=false;return Promise.resolve();}
    return new Promise(resolve=>{queued={id:generation,context:context!,bridge,resolve};pump();});
  }
  async function run(job:Job){
    try{
      const response=await job.bridge.invoke<SourceSectionPreparationResponse>('prepare_source_section',{requestId:job.context.requestId,sourceId:job.context.sourceId,preparationId:job.id,options:job.context.options});
      if(!current(job))return;
      if(response.request_id!==job.context.requestId||response.source_id!==job.context.sourceId||response.preparation_id!==job.id||!response.metadata)throw stale();
      if(!validMetadata(response.metadata))throw {code:'dxf.invalid_profile',message:'Invalid source preparation metadata.'};
      state.response=response;deps.onReady({...response,metadata:response.metadata});
    }catch(error){if(current(job))state.error=error;}
    finally{if(current(job))state.pending=false;job.resolve();active=null;pump();}
  }
  function dispose(){clear();disposed=true;}
  return {sync,clear,dispose};
}
