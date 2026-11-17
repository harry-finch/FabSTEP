import { PROFILE_PREVIEW_MAX_LOOPS, PROFILE_PREVIEW_MAX_SEGMENTS, type FaceProfileGeometry } from './face-profile-geometry.ts';
import type { PlanarFaceProfile, SectionProfile } from './types.ts';
export type FaceProfileWorker = Pick<Worker,'postMessage'|'terminate'|'onmessage'|'onerror'>;
const stale = () => ({ code:'engine.stale', message:'Stale planar face display.' });
const unavailable = () => ({ code:'engine.task', message:'Planar face display worker is unavailable.' });
/** One active calculation plus one latest pending profile: no unbounded worker queue. */
export function createFaceProfileGeometryWorker(worker:FaceProfileWorker) {
  type Input = {kind:'face';profile:PlanarFaceProfile}|{kind:'section';profile:SectionProfile};
  type Job = { id:number; input:Input; resolve:(geometry:FaceProfileGeometry|null)=>void; reject:(error:unknown)=>void };
  let serial = 0, active:Job|null = null, queued:Job|null = null, disposed = false;
  const dispatch = (job:Job) => {
    active = job;
    try { worker.postMessage({ id:job.id, ...job.input }); }
    catch (error) { active = null; job.reject(error); }
  };
  function clear() { active?.reject(stale()); queued?.reject(stale()); queued = null; }
  function dispose() { clear(); disposed = true; worker.terminate(); active = null; }
  worker.onmessage = (event:MessageEvent<{ id:number; geometry?:FaceProfileGeometry|null; error?:unknown }>) => {
    const job = active;
    if (disposed || !job || event.data.id !== job.id) return;
    if (event.data.geometry || (event.data.geometry === null && job.input.kind === 'section')) job.resolve(event.data.geometry); else job.reject(event.data.error ?? unavailable());
    active = null;
    if (queued) { const next = queued; queued = null; dispatch(next); }
  };
  worker.onerror = () => { active?.reject(unavailable()); queued?.reject(unavailable()); dispose(); };
  function enqueue(input:Input):Promise<FaceProfileGeometry|null> {
    if (disposed) return Promise.reject(unavailable());
    // Bound the structured clone before posting. Curve maths and path generation run in the worker.
    const groups = input.kind === 'face' ? [input.profile.loops] : input.profile.bodies.map(body=>body.loops);
    if (groups.length > 32 || groups.reduce((count,loops)=>count+loops.length,0) > PROFILE_PREVIEW_MAX_LOOPS || groups.reduce((count,loops)=>count+loops.reduce((sum,loop)=>sum+loop.segments.length,0),0) > PROFILE_PREVIEW_MAX_SEGMENTS) return Promise.reject({code:'dxf.too_complex',message:'Native planar profile exceeds display limits.'});
    return new Promise((resolve,reject) => {
      const job = { id:++serial, input, resolve, reject };
      if (active) { active.reject(stale()); queued?.reject(stale()); queued = job; }
      else dispatch(job);
    });
  }
  const prepare = (profile:PlanarFaceProfile) => enqueue({kind:'face',profile}) as Promise<FaceProfileGeometry>;
  const prepareSection = (profile:SectionProfile) => enqueue({kind:'section',profile});
  return { prepare, prepareSection, clear, dispose };
}
