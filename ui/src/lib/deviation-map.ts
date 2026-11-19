import type { ResultFaces } from './types.ts';

export type DeviationReport = {
  vertex_distances_mm: number[];
  face_maxima_mm: number[];
  min_mm: number;
  max_mm: number;
  budget_mm: number;
};
type Reply = { request_id:number; source_id:number; map_id:number; report:DeviationReport };
type Bridge = { invoke:<T>(command:string, args:Record<string,unknown>)=>Promise<T> };
export type DeviationState =
  | { kind:'off'|'pending'|'unavailable'|'overBudget' }
  | { kind:'ready'; report:DeviationReport };

function validReport(report:DeviationReport, document:ResultFaces) {
  return report && Number.isFinite(report.budget_mm) && report.budget_mm > 0
    && Number.isFinite(report.min_mm) && report.min_mm >= 0
    && Number.isFinite(report.max_mm) && report.max_mm >= report.min_mm
    && Array.isArray(report.vertex_distances_mm)
    && report.vertex_distances_mm.length === document.mesh.vertices.length
    && Array.isArray(report.face_maxima_mm) && report.face_maxima_mm.length === document.faces.length
    && [report.vertex_distances_mm, report.face_maxima_mm].every(samples =>
      samples.every(mm => Number.isFinite(mm) && mm >= report.min_mm && mm <= report.max_mm));
}

/** Owns only the display request. Export and face selection keep their own lifetimes. */
export function createDeviationMap(publish:(state:DeviationState)=>void) {
  let sequence = 0;
  let cancel: (()=>void) | null = null;
  function clear() {
    sequence++;
    cancel?.();
    cancel = null;
    publish({kind:'off'});
  }
  async function load(bridge:Bridge, document:ResultFaces, current:()=>boolean) {
    clear();
    const mapId = sequence;
    const args = {requestId:document.request_id, sourceId:document.source_id, mapId};
    cancel = () => { void bridge.invoke('cancel_deviation_map', args).catch(()=>{}); };
    publish({kind:'pending'});
    try {
      const reply = await bridge.invoke<Reply>('result_deviation_map', args);
      if (mapId !== sequence || !current()) return;
      if (reply.map_id !== mapId || reply.request_id !== args.requestId ||
          reply.source_id !== args.sourceId || !validReport(reply.report, document)) {
        publish({kind:'unavailable'});
      } else {
        publish({kind:'ready', report:reply.report});
      }
    } catch (error) {
      if (mapId === sequence && current()) {
        const budget = typeof error === 'object' && error !== null && 'code' in error
          && error.code === 'validation.distance_budget';
        publish({kind:budget ? 'overBudget' : 'unavailable'});
      }
    } finally {
      if (mapId === sequence) cancel = null;
    }
  }
  return {load, clear};
}
