import {test} from 'node:test';
import {strict as assert} from 'node:assert';
import {createStlPreviewWorker,type StlPreviewWorker,type StlPreviewGeometry} from '../src/lib/stl-preview-worker.ts';
function dense(count=100_001) {
  const bytes=new Uint8Array(84+50*count),view=new DataView(bytes.buffer);
  view.setUint32(80,count,true);
  for(let i=0;i<count;i++) {
    const start=84+50*i;
    // Independent triangle in XY, repeated only to exercise dense transport.
    view.setFloat32(start+24,1,true);view.setFloat32(start+40,1,true);
  }
  return bytes;
}
function geometry():StlPreviewGeometry {
  return {positions:new Float32Array(9),normals:new Float32Array(9),sourceTriangles:new Uint32Array(1),bounds:[[0,0,0],[1,1,0]],sphere:{center:[0.5,0.5,0],radius:1}};
}
function fixture() {
  const sent:{id:number;buffer:ArrayBuffer;unit:'mm'|'inch'}[]=[],transfers:unknown[]=[];let terminated=0;
  const port={postMessage:(message:typeof sent[number],transfer:unknown)=>{sent.push(message);transfers.push(transfer);},terminate:()=>{terminated++;},onmessage:null,onerror:null} as unknown as StlPreviewWorker;
  const worker=createStlPreviewWorker(port);
  const reply=(id:number)=>port.onmessage?.({data:{id,geometry:geometry()}} as MessageEvent);
  return {worker,port,sent,transfers,reply,terminated:()=>terminated};
}
test('dense display keeps one active and only the latest pending job without detaching the source',async()=>{
  const f=fixture(),source=dense();
  const first=f.worker.prepare(source,'mm').catch(e=>e),middle=f.worker.prepare(source,'inch').catch(e=>e),last=f.worker.prepare(source,'mm');
  assert.equal((await first).code,'engine.stale');assert.equal((await middle).code,'engine.stale');assert.equal(f.sent.length,1);
  assert.equal(source.byteLength,84+50*100_001);assert.notEqual(f.sent[0].buffer,source.buffer);
  assert.deepEqual(f.transfers[0],[f.sent[0].buffer]);
  f.reply(1);assert.equal(f.sent.length,2);assert.equal(f.sent[1].id,3);f.reply(1);f.reply(3);assert.ok(await last);f.worker.dispose();
});
test('replacement, unit changes, clear and disposal reject old display responses',async()=>{
  const f=fixture(),source=dense();
  const first=f.worker.prepare(source,'mm').catch(e=>e);f.worker.clear();assert.equal((await first).code,'engine.stale');
  const next=f.worker.prepare(source,'inch');f.reply(1);assert.equal(f.sent[1].unit,'inch');f.reply(2);assert.ok(await next);
  const active=f.worker.prepare(source,'mm').catch(e=>e),pending=f.worker.prepare(source,'inch').catch(e=>e);f.worker.dispose();
  assert.equal((await active).code,'engine.stale');assert.equal((await pending).code,'engine.stale');assert.equal(f.terminated(),1);
  f.reply(3);await assert.rejects(f.worker.prepare(source,'mm'),(e:{code:string})=>e.code==='engine.task');
});
test('ordinary, malformed and excessive sources are rejected before copy or dispatch',async()=>{
  const f=fixture();
  for(const bytes of [new Uint8Array(83),dense(100_000),dense(600_001),dense().subarray(1)]) {
    await assert.rejects(f.worker.prepare(bytes,'mm'),(e:{code:string})=>e.code==='preview.invalid_stl');
  }
  assert.equal(f.sent.length,0);f.worker.dispose();
});
test('display worker failure settles active and queued jobs and terminates once',async()=>{
  const f=fixture(),source=dense(),active=f.worker.prepare(source,'mm').catch(e=>e),pending=f.worker.prepare(source,'mm').catch(e=>e);
  f.port.onerror?.({} as ErrorEvent);assert.equal((await active).code,'engine.stale');assert.equal((await pending).code,'engine.task');assert.equal(f.terminated(),1);f.worker.dispose();assert.equal(f.terminated(),1);
});
test('actual dense worker transfers prepared normals, triangle identities and bounds with units',async t=>{
  const {Worker}=await import('node:worker_threads'),url=new URL('../src/lib/stl-preview.worker.ts',import.meta.url).href;
  const port=new Worker(`const {parentPort}=require('node:worker_threads');globalThis.self={postMessage:(value,options)=>parentPort.postMessage(value,options?.transfer??[]),set onmessage(handler){parentPort.on('message',data=>handler({data}));}};import(${JSON.stringify(url)});`,{eval:true});t.after(()=>port.terminate());
  const command=(message:unknown)=>new Promise<any>((resolve,reject)=>{port.once('message',resolve);port.once('error',reject);port.postMessage(message);});
  const source=dense(),reply=await command({id:1,buffer:source.buffer,unit:'inch'});
  assert.equal(reply.id,1);assert.equal(reply.geometry.positions.length,100_001*9);assert.equal(reply.geometry.normals.length,100_001*9);
  assert.equal(reply.geometry.sourceTriangles.at(-1),100_000);assert.deepEqual(reply.geometry.bounds[0],[0,0,0]);
  assert.ok(Math.abs(reply.geometry.bounds[1][0]-25.4)<1e-6);assert.ok(Math.abs(reply.geometry.bounds[1][1]-25.4)<1e-6);
  assert.equal(reply.geometry.normals[2],1);assert.ok(reply.geometry.sphere.radius>0);assert.equal(source.byteLength,84+50*100_001);
  const bad=new Uint8Array(84);new DataView(bad.buffer).setUint32(80,0xffffffff,true);
  assert.equal((await command({id:2,buffer:bad.buffer,unit:'mm'})).error.code,'preview.invalid_stl');
  new DataView(source.buffer).setFloat32(96,NaN,true);
  assert.equal((await command({id:3,buffer:source.buffer,unit:'mm'})).error.code,'preview.invalid_stl');
});
