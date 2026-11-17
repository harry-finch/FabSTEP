import { test } from 'node:test';
import { strict as assert } from 'node:assert';
import { faceProfileGeometry } from '../src/lib/face-profile-geometry.ts';
import type { FaceProfileSegment, PlanarFaceProfile } from '../src/lib/types.ts';
const profile=(segments:FaceProfileSegment[]):PlanarFaceProfile=>({face_id:0,solid_id:0,frame:{origin_mm:[10,20,30],x_axis:[1,0,0],y_axis:[0,0,1],normal:[0,-1,0]},loops:[{is_outer:true,segments}]});
const near=(a:number,b:number)=>assert.ok(Math.abs(a-b)<1e-10,`${a} ≠ ${b}`);
test('native line bounds preserve offsets and SVG flips Y only for display',()=>{
  const geometry=faceProfileGeometry(profile([{kind:'line',start_mm:[10,20],end_mm:[30,40]}]));
  assert.deepEqual(geometry.bounds,[[10,20],[30,40]]);assert.equal(geometry.width,20);assert.equal(geometry.height,20);assert.equal(geometry.paths[0].d,'M 10 20 L 30 40');assert.equal(geometry.viewBox,'8.4 -41.6 23.2 23.2');
});
test('circles and holes remain analytic and retain millimetre radii',()=>{
  const p=profile([{kind:'circle',center_mm:[5,-2],radius_mm:10}]);p.loops.push({is_outer:false,segments:[{kind:'circle',center_mm:[6,-3],radius_mm:2}]});
  const geometry=faceProfileGeometry(p);assert.deepEqual(geometry.bounds,[[-5,-12],[15,8]]);assert.equal(geometry.paths.length,2);assert.equal(geometry.paths[1].outer,false);assert.match(geometry.paths[0].d,/A 10 10/);assert.match(geometry.paths[1].d,/A 2 2/);
});
test('positive arc exceeding 180 degrees includes every swept cardinal extremum',()=>{
  const geometry=faceProfileGeometry(profile([{kind:'arc',center_mm:[2,3],radius_mm:4,start_angle_rad:Math.PI/4,sweep_angle_rad:3*Math.PI/2}]));
  near(geometry.bounds[0][0],-2);near(geometry.bounds[0][1],-1);near(geometry.bounds[1][0],2+Math.sqrt(8));near(geometry.bounds[1][1],7);assert.match(geometry.paths[0].d,/A 4 4 0 1 1/);
});
test('negative arc uses its signed sweep across zero with exact extrema',()=>{
  const geometry=faceProfileGeometry(profile([{kind:'arc',center_mm:[0,0],radius_mm:2,start_angle_rad:Math.PI/4,sweep_angle_rad:-Math.PI}]));
  near(geometry.bounds[0][0],-Math.sqrt(2));near(geometry.bounds[0][1],-2);near(geometry.bounds[1][0],2);near(geometry.bounds[1][1],Math.sqrt(2));assert.match(geometry.paths[0].d,/A 2 2 0 0 0/);
});
test('short arc across 2π does not include unswept extrema',()=>{
  const geometry=faceProfileGeometry(profile([{kind:'arc',center_mm:[0,0],radius_mm:2,start_angle_rad:7*Math.PI/4,sweep_angle_rad:Math.PI/2}]));
  near(geometry.bounds[0][0],Math.sqrt(2));near(geometry.bounds[1][0],2);near(geometry.height,2*Math.sqrt(2));assert.match(geometry.paths[0].d,/A 2 2 0 0 1/);
});
test('full signed arc is represented by two arcs without polygonisation',()=>{
  for(const sign of [1,-1]){const geometry=faceProfileGeometry(profile([{kind:'arc',center_mm:[0,0],radius_mm:3,start_angle_rad:0,sweep_angle_rad:sign*2*Math.PI}]));assert.equal((geometry.paths[0].d.match(/ A /g)||[]).length,2);near(geometry.width,6);near(geometry.height,6);}
});
test('empty, invalid, nonfinite and work-limit profiles are rejected',()=>{
  const cases=[profile([]),profile([{kind:'circle',center_mm:[0,0],radius_mm:-1}]),profile([{kind:'line',start_mm:[0,NaN],end_mm:[1,2]}]),profile([{kind:'arc',center_mm:[0,0],radius_mm:1,start_angle_rad:0,sweep_angle_rad:7}])];
  for(const p of cases)assert.throws(()=>faceProfileGeometry(p),(error:{code:string})=>error.code==='dxf.invalid_profile');
  const large=profile(Array.from({length:16385},()=>({kind:'line' as const,start_mm:[0,0] as [number,number],end_mm:[1,1] as [number,number]})));assert.throws(()=>faceProfileGeometry(large),(error:{code:string})=>error.code==='dxf.too_complex');
  const loops=profile([{kind:'circle',center_mm:[0,0],radius_mm:1}]);loops.loops=Array.from({length:1025},()=>loops.loops[0]);assert.throws(()=>faceProfileGeometry(loops),(error:{code:string})=>error.code==='dxf.too_complex');
});
