import type { ResultFaces } from './types.ts';

type Bridge = { invoke:<T>(command:string,args:Record<string,unknown>)=>Promise<T> };
type SuggestedRegion = { request_id:number; face_ids:number[] };
/** Read-only request. Caller owns the selection epoch; no conversion/export state is touched. */
export async function requestSuggestedRegion(bridge:Bridge, document:ResultFaces, faceId:number, isCurrent:()=>boolean):Promise<number[]|null> {
  if (!Number.isInteger(faceId) || !document.faces[faceId]) throw new Error('selection.invalid_region');
  const response = await bridge.invoke<SuggestedRegion>('suggest_region',{requestId:document.request_id,faceId});
  if (!isCurrent()) return null;
  if (response.request_id !== document.request_id || !Array.isArray(response.face_ids) || !response.face_ids.length || response.face_ids.length > document.faces.length) throw new Error('selection.invalid_region');
  const ids = response.face_ids;
  if (!ids.includes(faceId) || new Set(ids).size !== ids.length || ids.some(id=>!Number.isInteger(id)||!document.faces[id]||document.faces[id].solid_id!==document.faces[faceId].solid_id)) throw new Error('selection.invalid_region');
  return [...ids].sort((a,b)=>a-b);
}

/** Immediate single click; the second matching click requests the whole wall, without a second toggle. */
export class FaceClickSequence {
  private previous:{face:number;context:unknown;at:number}|null=null;
  click(face:number, context:unknown, at:number):'face'|'region' {
    const previous=this.previous;
    this.previous={face,context,at};
    if (previous && previous.face===face && previous.context===context && at>=previous.at && at-previous.at<=350) {
      this.previous=null; return 'region';
    }
    return 'face';
  }
  clear() {this.previous=null;}
}

/** At most one native calculation and one replacement request, even during rapid clicks. */
export class RegionSelectionQueue {
  private active=false;
  private queued:{run:()=>Promise<void>;discard:()=>void}|null=null;
  request(bridge:Bridge, document:ResultFaces, faceId:number, isCurrent:()=>boolean):Promise<number[]|null> {
    return new Promise((resolve,reject)=>{
      const job={discard:()=>resolve(null),run:async()=>{
        if(!isCurrent()){resolve(null);return;}
        try {resolve(await requestSuggestedRegion(bridge,document,faceId,isCurrent));}catch(error){reject(error);}
      }};
      if(this.active){this.queued?.discard();this.queued=job;}
      else void this.run(job);
    });
  }
  private async run(job:{run:()=>Promise<void>;discard:()=>void}) {
    this.active=true;
    await job.run();
    const next=this.queued;this.queued=null;
    if(next)void this.run(next);else this.active=false;
  }
}
