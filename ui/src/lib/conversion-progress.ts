import type { SourceBridge } from './source.ts';

export const conversionPhases = ['guidance_inspection', 'analysis', 'source_separation', 'body', 'recognition', 'source_validation', 'build', 'audit_tessellation', 'audit_comparison', 'combine', 'result_inspection', 'finish'] as const;
export type ConversionPhase = typeof conversionPhases[number];
export type ConversionProgress = { request_id:number; source_id:number; phase:ConversionPhase; body_index:number|null; body_count:number|null };

// Stages describe observed operations, never fractions of elapsed/remaining time.
export function conversionStage(phase:ConversionPhase):number {
  if (phase === 'guidance_inspection' || phase === 'analysis' || phase === 'source_separation') return 0;
  if (phase === 'combine' || phase === 'result_inspection' || phase === 'finish') return 2;
  return 1;
}
function valid(value:unknown, requestId:number, sourceId:number):value is ConversionProgress {
  if (!value || typeof value !== 'object') return false;
  const p = value as ConversionProgress;
  const index = p.body_index;
  const count = p.body_count;
  return p.request_id === requestId && p.source_id === sourceId
    && conversionPhases.includes(p.phase)
    && (index === null || (Number.isSafeInteger(index) && index >= 0 && index < 32))
    && (count === null || (Number.isSafeInteger(count) && count > 0 && count <= 32 && (index === null || index < count)));
}
export function watchConversionProgress(bridge:SourceBridge, requestId:number, sourceId:number,
  publish:(progress:ConversionProgress|null)=>void,
  schedule:(callback:()=>void)=>unknown = callback=>setTimeout(callback,350),
  clear:(handle:unknown)=>void = handle=>clearTimeout(handle as ReturnType<typeof setTimeout>),
) {
  let active = true;
  let timer:unknown;
  async function poll() {
    try {
      const response = await bridge.invoke<unknown>('get_conversion_progress', {requestId,sourceId});
      if (active) publish(valid(response,requestId,sourceId) ? response : null);
    } catch {
      // Observation cannot fail a conversion or leave a stale phase displayed.
      if (active) publish(null);
    } finally {
      if (active) timer = schedule(()=>{timer=undefined;void poll();});
    }
  }
  void poll();
  return () => { active=false; if(timer!==undefined) clear(timer); };
}
