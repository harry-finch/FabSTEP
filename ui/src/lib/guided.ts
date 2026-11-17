import type { GuidedFamily, Options, Result, ResultFaces } from './types.ts';
import type { StepSelection } from './step-selection.ts';

/** Preparation deliberately replaces automatic reconstruction with a faceted base. */
export function canPrepareCorrection(result:Pick<Result, 'request_id' | 'settings'> | null, requestId:number, mode:Options['reconstruction'], nativeAvailable:boolean, pending:boolean):boolean {
  return nativeAvailable && !pending && !!result && result.request_id === requestId
    && mode !== 'planar' && mode !== 'auto' && result.settings.reconstruction === mode;
}

/** Eligibility is only a UI guard; the engine independently resolves and proves the wall. */
export function guidedFaces(document:ResultFaces|null, selection:StepSelection|null, requestId:number, sourceId:number|null, hintCount = 0):number[]|null {
  if (!Number.isInteger(hintCount) || hintCount < 0 || hintCount >= 16 || !document || document.request_id !== requestId || document.source_id !== sourceId || document.source_correspondence.status !== 'exact' || !selection?.faces.length || selection.faces.length > 100_000 || selection.source.status !== 'exact' || selection.solids.length !== 1) return null;
  const selected = new Set(selection.faces);
  if (selected.size !== selection.faces.length || selection.faces.some(id => !Number.isInteger(id) || !document.faces[id])) return null;
  const rows = document.source_correspondence.faces.filter(row => selected.has(row.face_id));
  if (rows.length !== selected.size || new Set(rows.map(row => row.analysis_body_id)).size !== 1) return null;
  return [...selected].sort((a,b) => a-b);
}

export function acceptsGuidedResult(requestId:number, sourceId:number, current:number, currentSource:number|null, responseId:number):boolean {
  return requestId === current && sourceId === currentSource && responseId === requestId;
}

/** Accepted hints are labelled from the native report, never the current selector. */
export function guidedFamilyLabel(family:GuidedFamily):'guidance.autoLabel'|'guidance.torusLabel'|'guidance.planeLabel'|'guidance.cylinderLabel'|'guidance.coneLabel' {
  return family === 'auto' ? 'guidance.autoLabel' : family === 'torus' ? 'guidance.torusLabel' : family === 'plane' ? 'guidance.planeLabel' : family === 'cone' ? 'guidance.coneLabel' : 'guidance.cylinderLabel';
}

/** A refusal belongs to a guided attempt only while its request is current. */
export function isGuidedRefusal(attempt:{requestId:number;status:'pending'|'success'|'refused'}|null, requestId:number, hasError:boolean):boolean {
  return hasError && attempt?.requestId === requestId && attempt.status === 'refused';
}
