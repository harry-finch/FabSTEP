import { test } from 'node:test';
import { strict as assert } from 'node:assert';
import { conversionStage, watchConversionProgress, type ConversionProgress } from '../src/lib/conversion-progress.ts';
import type { SourceBridge } from '../src/lib/source.ts';
const tick = () => new Promise(resolve=>setImmediate(resolve));
const sample:ConversionProgress = {request_id:10,source_id:3,phase:'recognition',body_index:1,body_count:null};
function fixture() {
  const pending:{resolve:(value:unknown)=>void;reject:(value:unknown)=>void}[]=[];
  const updates:(ConversionProgress|null)[]=[];
  const timers=new Map<number,()=>void>(); let next=0;
  const bridge:SourceBridge={invoke:<T>()=>new Promise<T>((resolve,reject)=>pending.push({resolve:value=>resolve(value as T),reject}))};
  const stop=watchConversionProgress(bridge,10,3,value=>updates.push(value),callback=>{timers.set(++next,callback);return next;},id=>timers.delete(id as number));
  const flush=()=>{const callbacks=[...timers.values()];timers.clear();callbacks.forEach(fn=>fn());};
  return {pending,updates,timers,stop,flush};
}
test('progress polling waits for its reply, publishes actual body index, and stops its timer',async()=>{
  const f=fixture();assert.equal(f.pending.length,1);assert.equal(f.timers.size,0);
  f.pending[0].resolve(sample);await tick();assert.deepEqual(f.updates,[sample]);assert.equal(f.timers.size,1);
  f.flush();assert.equal(f.pending.length,2);assert.equal(f.timers.size,0);
  f.pending[1].resolve({...sample,phase:'build'});await tick();f.stop();assert.equal(f.timers.size,0);
});
test('stopped conversion cannot publish a late reply or restart polling',async()=>{
  const f=fixture();f.stop();f.pending[0].resolve(sample);await tick();assert.deepEqual(f.updates,[]);assert.equal(f.timers.size,0);
});
test('stale or malformed observations and IPC failures clear the phase without failing conversion',async()=>{
  const f=fixture();
  const invalid=[{...sample,request_id:9},{...sample,source_id:4},{...sample,phase:'invented'}, {...sample,body_index:32},{...sample,body_count:1},{...sample,body_count:NaN},null];
  for (const value of invalid) {f.pending.at(-1)!.resolve(value);await tick();assert.equal(f.updates.at(-1),null);f.flush();}
  f.pending.at(-1)!.reject(new Error('observation unavailable'));await tick();assert.equal(f.updates.at(-1),null);f.flush();
  f.pending.at(-1)!.resolve(sample);await tick();assert.deepEqual(f.updates.at(-1),sample);f.stop();
});
test('document stages keep per-body recognition/build/audit in the reconstruction stage',()=>{
  assert.equal(conversionStage('guidance_inspection'),0);assert.equal(conversionStage('analysis'),0);assert.equal(conversionStage('source_separation'),0);
  for (const phase of ['body','recognition','source_validation','build','audit_tessellation','audit_comparison'] as const) assert.equal(conversionStage(phase),1);
  for (const phase of ['combine','result_inspection','finish'] as const) assert.equal(conversionStage(phase),2);
});
