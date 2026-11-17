import {test} from 'node:test';
import {strict as assert} from 'node:assert';
import {faceProfileGeometry,PROFILE_PREVIEW_MAX_SEGMENTS,sectionProfileGeometry} from '../src/lib/face-profile-geometry.ts';
import {createFaceProfileGeometryWorker,type FaceProfileWorker} from '../src/lib/face-profile-worker.ts';
import type {FaceProfileSegment,PlanarFaceProfile,SectionProfile} from '../src/lib/types.ts';
const TAU=2*Math.PI;
type Ellipse=Extract<FaceProfileSegment,{kind:'ellipse'}>;
const ellipse=(values:Partial<Ellipse>={}):Ellipse=>({kind:'ellipse',center_mm:[7,-11],major_axis_mm:[3,4],ratio:0.5,start_param_rad:0,sweep_param_rad:TAU,...values});
const profile=(segments:FaceProfileSegment[]):PlanarFaceProfile=>({face_id:0,solid_id:0,frame:{origin_mm:[0,0,0],x_axis:[1,0,0],y_axis:[0,1,0],normal:[0,0,1]},loops:[{is_outer:true,segments}]});
const near=(a:number,b:number)=>assert.ok(Math.abs(a-b)<=1e-10*Math.max(1,Math.abs(b)),`${a} != ${b}`);
const bounds=(geometry:ReturnType<typeof faceProfileGeometry>,expected:number[][])=>{for(let i=0;i<2;i++)for(let j=0;j<2;j++)near(geometry.bounds[i][j],expected[i][j]);};
test('full rotated ellipses retain exact SVG radii/rotation and analytic extrema in both directions',()=>{
  for(const sign of [1,-1]){
    const geometry=faceProfileGeometry(profile([ellipse({start_param_rad:1.234,sweep_param_rad:sign*TAU})]));
    bounds(geometry,[[7-Math.sqrt(13),-11-Math.sqrt(18.25)],[7+Math.sqrt(13),-11+Math.sqrt(18.25)]]);
    const path=geometry.paths[0].d;assert.equal((path.match(/ A /g)||[]).length,2);assert.ok(path.includes(`A 5 2.5 ${Math.atan2(4,3)*180/Math.PI} 0 ${sign>0?1:0}`));assert.equal(path.includes(' L '),false);
    const tokens=path.split(' ');assert.deepEqual(tokens.slice(1,3),tokens.slice(-2));
  }
});
test('rotated partial ellipse includes an internal coordinate extremum and excludes unswept extrema',()=>{
  const ccw=faceProfileGeometry(profile([ellipse({sweep_param_rad:Math.PI/2})]));bounds(ccw,[[5,-9.5],[10,-11+Math.sqrt(18.25)]]);assert.match(ccw.paths[0].d,/ 0 1 /);
  const cw=faceProfileGeometry(profile([ellipse({sweep_param_rad:-Math.PI/2})]));bounds(cw,[[9,-12.5],[7+Math.sqrt(13),-7]]);assert.match(cw.paths[0].d,/ 0 0 /);
});
test('partial ellipses crossing the parameter seam use the signed interval for exact bounds',()=>{
  for(const [start,sweep] of [[7*Math.PI/4,Math.PI/2],[Math.PI/4,-Math.PI/2]]){
    const geometry=faceProfileGeometry(profile([ellipse({center_mm:[0,0],major_axis_mm:[4,0],start_param_rad:start,sweep_param_rad:sweep})]));bounds(geometry,[[2*Math.SQRT2,-Math.SQRT2],[4,Math.SQRT2]]);
    assert.ok(geometry.paths[0].d.includes(`A 4 2 0 0 ${sweep>0?1:0}`));
  }
});
test('large signed ellipse arcs retain SVG large-arc flags and vertical major axes rotate the ellipse',()=>{
  for(const sign of [1,-1]){
    const geometry=faceProfileGeometry(profile([ellipse({center_mm:[0,0],major_axis_mm:[0,4],sweep_param_rad:sign*1.5*Math.PI})]));bounds(geometry,[[-2,-4],[2,4]]);assert.ok(geometry.paths[0].d.includes(`A 4 2 90 1 ${sign>0?1:0}`));assert.equal((geometry.paths[0].d.match(/ A /g)||[]).length,1);
  }
});
test('near-full ellipses remain partial and circular ratios remain exact ellipse curves',()=>{
  const almost=faceProfileGeometry(profile([ellipse({start_param_rad:0.2,sweep_param_rad:TAU-1e-10})]));assert.equal((almost.paths[0].d.match(/ A /g)||[]).length,1);const tokens=almost.paths[0].d.split(' ');assert.notDeepEqual(tokens.slice(1,3),tokens.slice(-2));
  const circular=faceProfileGeometry(profile([ellipse({center_mm:[0,0],major_axis_mm:[5,0],ratio:1})]));bounds(circular,[[-5,-5],[5,5]]);assert.match(circular.paths[0].d,/A 5 5 0/);
});
test('finite huge start parameters preserve a nonzero sweep without overflow or angle addition loss',()=>{
  const start=Number.MAX_VALUE,geometry=faceProfileGeometry(profile([ellipse({start_param_rad:start,sweep_param_rad:0.25})])),tokens=geometry.paths[0].d.split(' ');
  assert.equal(geometry.paths[0].d.includes('NaN'),false);assert.equal(geometry.paths[0].d.includes('Infinity'),false);assert.notDeepEqual(tokens.slice(1,3),tokens.slice(-2));near(Number(tokens[1]),7+3*Math.cos(start)-2*Math.sin(start));near(Number(tokens[2]),-11+4*Math.cos(start)+1.5*Math.sin(start));
});
test('invalid ellipse radii, ratios, parameters, coordinates and overflow are refused with a stable code',()=>{
  const bad:Ellipse[]=[
    ...[0,-1,1.01,NaN,Infinity].map(ratio=>ellipse({ratio})),
    ...[0,TAU+1e-12,NaN,Infinity].map(sweep_param_rad=>ellipse({sweep_param_rad})),
    ellipse({start_param_rad:Infinity}),ellipse({major_axis_mm:[0,0]}),ellipse({major_axis_mm:[NaN,1]}),ellipse({major_axis_mm:[Number.MAX_VALUE,Number.MAX_VALUE]}),ellipse({center_mm:[Infinity,0]}),
    ellipse({major_axis_mm:[1e-300,0],ratio:Number.MIN_VALUE}),ellipse({major_axis_mm:[Number.MAX_VALUE,0],center_mm:[Number.MAX_VALUE,0]}),
    ellipse({major_axis_mm:[8e307,0],center_mm:[8e307,0],ratio:0.1}),
    ellipse({center_mm:null as unknown as [number,number]}),ellipse({major_axis_mm:[1,2,3] as unknown as [number,number]})
  ];
  for(const segment of bad)assert.throws(()=>faceProfileGeometry(profile([segment])),(error:{code:string})=>error.code==='dxf.invalid_profile');
});
test('ellipse segments keep the existing shared work budget even when a full turn uses two SVG arcs',async()=>{
  const maximum=profile(Array.from({length:PROFILE_PREVIEW_MAX_SEGMENTS},()=>ellipse({major_axis_mm:[4,0],center_mm:[0,0]})));const geometry=faceProfileGeometry(maximum);assert.equal(geometry.paths.length,1);assert.equal((geometry.paths[0].d.match(/ A /g)||[]).length,2*PROFILE_PREVIEW_MAX_SEGMENTS);
  const over=profile([...maximum.loops[0].segments,ellipse()]);let posts=0;const port={postMessage:()=>{posts++;},terminate:()=>{},onmessage:null,onerror:null} as unknown as FaceProfileWorker,worker=createFaceProfileGeometryWorker(port);
  await assert.rejects(worker.prepare(over),(error:{code:string})=>error.code==='dxf.too_complex');assert.equal(posts,0);worker.dispose();
});
test('section ellipse paths preserve holes, component identities and all-empty selected bodies',()=>{
  const section:SectionProfile={frame:profile([]).frame,bodies:[{solid_id:2,loops:[{component_id:3,is_outer:true,segments:[ellipse()]},{component_id:3,is_outer:false,segments:[ellipse({ratio:0.25,major_axis_mm:[1,0]})]}]},{solid_id:5,loops:[]}]};
  const geometry=sectionProfileGeometry(section)!;assert.deepEqual(geometry.paths.map(path=>[path.solidId,path.componentId,path.outer]),[[2,3,true],[2,3,false]]);assert.equal(section.bodies[1].loops.length,0);assert.ok(geometry.paths.every(path=>path.d.includes(' A ')));
});
test('actual worker transports analytic ellipse face/section DTOs and stable refusals without sampling',async t=>{
  const {Worker}=await import('node:worker_threads'),url=new URL('../src/lib/face-profile.worker.ts',import.meta.url).href;
  const worker=new Worker(`const {parentPort}=require('node:worker_threads');globalThis.self={postMessage:value=>parentPort.postMessage(value),set onmessage(handler){parentPort.on('message',data=>handler({data}));}};import(${JSON.stringify(url)});`,{eval:true});t.after(()=>worker.terminate());
  const command=(message:unknown)=>new Promise<any>((resolve,reject)=>{worker.once('message',resolve);worker.once('error',reject);worker.postMessage(message);});
  const native=profile([ellipse({sweep_param_rad:-1.5*Math.PI})]),face=await command({id:1,kind:'face',profile:native});assert.equal(face.id,1);assert.deepEqual(face.geometry,faceProfileGeometry(native));assert.equal(face.geometry.paths[0].d.includes(' L '),false);
  const section:SectionProfile={frame:native.frame,bodies:[{solid_id:3,loops:[{component_id:2,is_outer:true,segments:[ellipse()]}]},{solid_id:4,loops:[]}]};const reply=await command({id:2,kind:'section',profile:section});assert.deepEqual(reply.geometry,sectionProfileGeometry(section));assert.equal(reply.geometry.paths[0].solidId,3);assert.equal(reply.geometry.paths[0].componentId,2);
  const refused=await command({id:3,kind:'face',profile:profile([ellipse({ratio:0})])});assert.equal(refused.error.code,'dxf.invalid_profile');
});
