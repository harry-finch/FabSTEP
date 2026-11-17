import {test} from 'node:test';
import {strict as assert} from 'node:assert';
import {sectionProfileGeometry} from '../src/lib/face-profile-geometry.ts';
import {sectionPlaneOverlay} from '../src/lib/section-plane.ts';
import {createFaceProfileGeometryWorker,type FaceProfileWorker} from '../src/lib/face-profile-worker.ts';
import type {ProfileFrame,SectionProfile} from '../src/lib/types.ts';
const frame:ProfileFrame={origin_mm:[10,20,30],x_axis:[1,0,0],y_axis:[0,1,0],normal:[0,0,1]};
const circle=(component_id:number,is_outer:boolean,x:number,radius_mm:number)=>({component_id,is_outer,segments:[{kind:'circle' as const,center_mm:[x,0] as [number,number],radius_mm}]});
const profile:SectionProfile={frame,bodies:[{solid_id:0,loops:[circle(0,true,0,4),circle(0,false,0,1),circle(1,true,12,2)]},{solid_id:2,loops:[circle(0,true,24,3)]},{solid_id:5,loops:[]}]};
test('section preview preserves every solid, component, outer boundary and hole in one frame',()=>{
  const geometry=sectionProfileGeometry(profile)!;assert.equal(geometry.paths.length,4);assert.deepEqual(geometry.bounds,[[-4,-4],[27,4]]);assert.deepEqual(geometry.paths.map(path=>[path.solidId,path.componentId,path.outer]),[[0,0,true],[0,0,false],[0,1,true],[2,0,true]]);assert.match(geometry.paths[1].d,/A 1 1/);assert.equal(profile.bodies[2].loops.length,0);
});
test('all-empty sections have no fake bounds; invalid component ownership is refused',()=>{
  assert.equal(sectionProfileGeometry({frame,bodies:[{solid_id:0,loops:[]}]}),null);
  const bad=structuredClone(profile);bad.bodies[0].loops[0].is_outer=false;assert.throws(()=>sectionProfileGeometry(bad),(error:{code:string})=>error.code==='dxf.invalid_profile');
});
test('shared worker manager bounds the total section before cloning multiple bodies',async()=>{
  let posts=0;const port={postMessage:()=>{posts++;},terminate:()=>{},onmessage:null,onerror:null} as unknown as FaceProfileWorker,worker=createFaceProfileGeometryWorker(port);
  const large:SectionProfile={frame,bodies:[0,1].map(solid_id=>({solid_id,loops:[{component_id:0,is_outer:true,segments:Array.from({length:9000},()=>({kind:'circle' as const,center_mm:[0,0] as [number,number],radius_mm:1}))}]}))};
  await assert.rejects(worker.prepareSection(large),(error:{code:string})=>error.code==='dxf.too_complex');assert.equal(posts,0);worker.dispose();
});
test('actual shared worker prepares section identities, analytic circles and empty profiles',async t=>{
  const {Worker}=await import('node:worker_threads'),url=new URL('../src/lib/face-profile.worker.ts',import.meta.url).href;
  const worker=new Worker(`const {parentPort}=require('node:worker_threads');globalThis.self={postMessage:value=>parentPort.postMessage(value),set onmessage(handler){parentPort.on('message',data=>handler({data}));}};import(${JSON.stringify(url)});`,{eval:true});t.after(()=>worker.terminate());
  const command=(message:unknown)=>new Promise<any>((resolve,reject)=>{worker.once('message',resolve);worker.once('error',reject);worker.postMessage(message);});
  const reply=await command({id:4,kind:'section',profile});assert.equal(reply.id,4);assert.equal(reply.geometry.paths[3].solidId,2);assert.equal(reply.geometry.paths[2].componentId,1);assert.match(reply.geometry.paths[0].d,/A 4 4/);
  const empty=await command({id:5,kind:'section',profile:{frame,bodies:[{solid_id:0,loops:[]}]}});assert.equal(empty.id,5);assert.equal(empty.geometry,null);
});
test('3D plane display follows the exact native frame and never mutates model bounds',()=>{
  const bounds=[[0,0,0],[20,30,40]],before=structuredClone(bounds),plane=sectionPlaneOverlay(frame,bounds)!;assert.equal(plane.positions.length,18);assert.equal(plane.outline.length,24);for(let i=2;i<plane.positions.length;i+=3)assert.equal(plane.positions[i],30);assert.deepEqual(plane.center,[10,15,30]);assert.deepEqual(plane.normal,[0,0,1]);assert.deepEqual(bounds,before);
  const oblique:ProfileFrame={origin_mm:[2,3,4],x_axis:[Math.SQRT1_2,Math.SQRT1_2,0],y_axis:[0,0,1],normal:[Math.SQRT1_2,-Math.SQRT1_2,0]},tilted=sectionPlaneOverlay(oblique,bounds)!;
  for(let i=0;i<tilted.positions.length;i+=3){const distance=tilted.positions.slice(i,i+3).reduce((sum,value,index)=>sum+(value-oblique.origin_mm[index])*oblique.normal[index],0);assert.ok(Math.abs(distance)<1e-12);}
});
