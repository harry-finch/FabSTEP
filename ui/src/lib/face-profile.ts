import type { SourceBridge } from './source.ts';
import type { FaceProfileResponse, PlanarFaceProfile } from './types.ts';
import type { FaceProfileGeometry } from './face-profile-geometry.ts';

export type FaceProfileContext = { requestId:number; sourceId:number; faceId:number };
export type FaceProfileState = { response:FaceProfileResponse|null; geometry:FaceProfileGeometry|null; pending:boolean; exporting:boolean; error:unknown|null; note:'saved'|'cancelled'|null };
type Dependencies = { native:()=>SourceBridge|undefined; context:()=>FaceProfileContext|null; name:()=>string; prepare:(profile:PlanarFaceProfile)=>Promise<FaceProfileGeometry>; clearPrepared?:()=>void; initialSelectionId?:number };
const same = (a:FaceProfileContext|null,b:FaceProfileContext|null) => a?.requestId === b?.requestId && a?.sourceId === b?.sourceId && a?.faceId === b?.faceId;

/** Read-only extraction/export lifecycle. The live facade never owns or invalidates the STEP. */
export function createFaceProfiles(state:FaceProfileState, deps:Dependencies) {
  let context:FaceProfileContext|null = null, selectionId = deps.initialSelectionId ?? 0, disposed = false;
  function current(id:number, captured:FaceProfileContext) { return !disposed && id === selectionId && same(context,captured) && same(deps.context(),captured); }
  function reset() { state.response = null; state.geometry = null; state.pending = false; state.exporting = false; state.error = null; state.note = null; deps.clearPrepared?.(); }
  function clear() {
    const previous = context;
    context = null; selectionId++; reset();
    const bridge = deps.native();
    if (previous && bridge) void bridge.invoke<void>('set_face_profile_context',{ ...previous, selectionId, faceId:null }).catch(() => { /* Native revision guards also reject an invalidated result. */ });
  }
  async function sync() {
    if (disposed) return;
    const next = deps.context();
    if (same(context,next)) return;
    if (!next) { clear(); return; }
    context = { ...next }; const captured = context, id = ++selectionId;
    reset(); state.pending = true;
    const bridge = deps.native();
    if (!bridge) { state.pending = false; return; }
    const args = { ...captured, selectionId:id };
    try {
      await bridge.invoke<void>('set_face_profile_context',args);
      if (!current(id,captured)) return;
      const response = await bridge.invoke<FaceProfileResponse>('face_profile',args);
      if (!current(id,captured)) return;
      if (response.request_id !== captured.requestId || response.source_id !== captured.sourceId || response.selection_id !== id || response.profile.face_id !== captured.faceId) throw { code:'engine.stale', message:'Stale planar face profile.' };
      const geometry = await deps.prepare(response.profile);
      if (!current(id,captured)) return;
      state.geometry = geometry;
      state.response = response;
    } catch (error) { if (current(id,captured)) state.error = error; }
    finally { if (current(id,captured)) state.pending = false; }
  }
  function canExport() {
    const response = state.response;
    return !disposed && !!deps.native() && !!context && same(context,deps.context()) && !state.pending && !state.exporting && !!state.geometry && !!response && response.request_id === context.requestId && response.source_id === context.sourceId && response.selection_id === selectionId && response.profile.face_id === context.faceId;
  }
  async function exportDxf() {
    if (!canExport()) return;
    const captured = { ...context! }, id = selectionId;
    state.exporting = true; state.error = null; state.note = null;
    try {
      const suggestedName = `${deps.name().replace(/\.(stl|3mf)$/i,'')}-face-${captured.faceId+1}.dxf`;
      const path = await deps.native()!.invoke<string|null>('export_face_dxf',{ ...captured, selectionId:id, suggestedName });
      if (current(id,captured)) state.note = path ? 'saved' : 'cancelled';
    } catch (error) {
      if (current(id,captured)) {
        state.error = error;
        if ((error as { code?:string })?.code === 'engine.stale') { state.response = null; state.geometry = null; }
      }
    } finally { if (current(id,captured)) state.exporting = false; }
  }
  function dispose() { clear(); disposed = true; }
  return { sync, clear, canExport, exportDxf, dispose };
}
