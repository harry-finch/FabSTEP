import { largeStlTriangles } from './large-stl.ts';
export type StlPreviewGeometry = {
  positions: Float32Array;
  normals: Float32Array;
  sourceTriangles: Uint32Array;
  bounds: [number[], number[]];
  sphere: { center: number[]; radius: number };
};
export type StlPreviewWorker = Pick<Worker, 'postMessage' | 'terminate' | 'onmessage' | 'onerror'>;
const stale = () => ({ code:'engine.stale', message:'Stale source display.' });
const unavailable = () => ({ code:'engine.task', message:'Source display worker is unavailable.' });
/** Dense binary display only: one active job and one latest pending source. */
export function createStlPreviewWorker(worker:StlPreviewWorker) {
  type Job = { id:number; bytes:Uint8Array; unit:'mm'|'inch'; resolve:(geometry:StlPreviewGeometry)=>void; reject:(error:unknown)=>void };
  let serial=0, active:Job|null=null, queued:Job|null=null, disposed=false;
  function dispatch(job:Job) {
    active=job;
    try {
      // Source bytes stay owned by the document. Only this bounded copy is transferred.
      const buffer=job.bytes.buffer.slice(job.bytes.byteOffset,job.bytes.byteOffset+job.bytes.byteLength) as ArrayBuffer;
      worker.postMessage({id:job.id,buffer,unit:job.unit},[buffer]);
    } catch(error) {active=null;job.reject(error);}
  }
  function clear() {active?.reject(stale());queued?.reject(stale());queued=null;}
  function dispose() {if(disposed)return;clear();disposed=true;worker.terminate();active=null;}
  worker.onmessage=(event:MessageEvent<{id:number;geometry?:StlPreviewGeometry;error?:unknown}>)=>{
    const job=active;
    if(disposed||!job||event.data.id!==job.id)return;
    if(event.data.geometry)job.resolve(event.data.geometry);else job.reject(event.data.error??unavailable());
    active=null;
    if(queued){const next=queued;queued=null;dispatch(next);}
  };
  worker.onerror=()=>{active?.reject(unavailable());queued?.reject(unavailable());dispose();};
  function prepare(bytes:Uint8Array,unit:'mm'|'inch'):Promise<StlPreviewGeometry> {
    if(disposed)return Promise.reject(unavailable());
    if(largeStlTriangles(bytes,'stl')===null)return Promise.reject({code:'preview.invalid_stl',message:'Dense source display requires a bounded binary STL.'});
    return new Promise((resolve,reject)=>{
      const job={id:++serial,bytes,unit,resolve,reject};
      if(active){active.reject(stale());queued?.reject(stale());queued=job;}else dispatch(job);
    });
  }
  return {prepare,clear,dispose};
}
