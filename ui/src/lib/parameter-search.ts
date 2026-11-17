import type { Options } from './types.ts';

export type ParameterTrial = { cylinder_fit_tolerance_mm:number; outcome:'validated'|'rejected'; faces:number|null; curved_faces:number|null; max_deviation_bound_mm:number|null; max_sampled_distance_mm:number|null; error:{code:string;message:string}|null };
export type ParameterSearchReport = { settings:Omit<Options,'input_unit'>; trials:ParameterTrial[]; best_index:number|null };
export type ParameterSearchResponse = { request_id:number; report:ParameterSearchReport };
export type SearchToken = { sequence:number; requestId:number; sourceId:number; options:Options };
/** A proposal belongs to the exact input and invocation, including repeated searches. */
export class ParameterSearchReplies {
  private sequence = 0;
  clear() { this.sequence++; }
  begin(requestId:number, sourceId:number, options:Options):SearchToken {
    return { sequence:++this.sequence, requestId, sourceId, options:{...options} };
  }
  accepts(token:SearchToken, requestId:number, sourceId:number|null, options:Options, responseId = requestId):boolean {
    return token.sequence === this.sequence && token.requestId === requestId && token.sourceId === sourceId && responseId === requestId && (Object.keys(token.options) as (keyof Options)[]).every(key => Object.is(token.options[key],options[key]));
  }
}
export function proposedTolerance(report:ParameterSearchReport):number|null {
  const index = report.best_index;
  const trial = index !== null && Number.isInteger(index) && index >= 0 ? report.trials[index] : null;
  return trial?.outcome === 'validated' && Number.isFinite(trial.cylinder_fit_tolerance_mm) && trial.cylinder_fit_tolerance_mm > 0 ? trial.cylinder_fit_tolerance_mm : null;
}
