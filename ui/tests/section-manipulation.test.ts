import {test} from 'node:test';
import {strict as assert} from 'node:assert';
import {Object3D,PerspectiveCamera,Quaternion,Scene,Vector3} from 'three';
import {TransformControls} from 'three/examples/jsm/controls/TransformControls.js';
import {claimSectionManipulator,cloneSectionInput,configureSectionManipulator,localSectionFrame,offsetSectionFrame,sectionControlPointer,sectionFrameFromPose,sectionManipulationPose,SectionManipulationGesture,transformedSectionInput} from '../src/lib/section-manipulation.ts';
import type {ProfileFrame,SectionInput} from '../src/lib/types.ts';
const frame:ProfileFrame={origin_mm:[100,200,30],x_axis:[1,0,0],y_axis:[0,1,0],normal:[0,0,1]};
const input:SectionInput={basis:'xy',face_id:null,offset_mm:30,solid_ids:[0,2]};
const near=(a:number,b:number)=>assert.ok(Math.abs(a-b)<1e-10,`${a} != ${b}`);
test('pivot is on the plane near the translated part rather than at the world origin',()=>{
  const pose=sectionManipulationPose(frame,[[1000,2000,0],[1020,2040,50]])!;assert.deepEqual(pose.position.toArray(),[1010,2020,30]);const rebased=sectionFrameFromPose(pose.position,pose.quaternion)!;near(new Vector3(...rebased.origin_mm).sub(new Vector3(...frame.origin_mm)).dot(new Vector3(...frame.normal)),0);
  const turned=sectionFrameFromPose(pose.position,pose.quaternion.clone().multiply(new Quaternion().setFromAxisAngle(new Vector3(1,0,0),Math.PI/4)))!;assert.deepEqual(turned.origin_mm,rebased.origin_mm);near(new Vector3(...turned.normal).dot(new Vector3(...frame.normal)),Math.SQRT1_2);
});
test('arbitrary quaternion rotations serialize an orthonormal right-handed frame',()=>{
  const rotation=new Quaternion().setFromAxisAngle(new Vector3(1,2,3).normalize(),1.234),result=sectionFrameFromPose(new Vector3(4,5,6),rotation)!;
  const x=new Vector3(...result.x_axis),y=new Vector3(...result.y_axis),n=new Vector3(...result.normal);near(x.length(),1);near(y.length(),1);near(n.length(),1);near(x.dot(y),0);near(x.dot(n),0);near(y.dot(n),0);near(x.clone().cross(y).dot(n),1);assert.deepEqual(rotation.toArray(),new Quaternion().setFromAxisAngle(new Vector3(1,2,3).normalize(),1.234).toArray());
});
test('gizmo frame bakes the visible offset exactly once and preserves chosen solid identities',()=>{
  const visible=localSectionFrame(input,null)!,pose=sectionManipulationPose(visible,[[0,0,0],[20,20,40]])!;pose.position.add(new Vector3(...visible.normal).multiplyScalar(7));const desired=sectionFrameFromPose(pose.position,pose.quaternion)!,custom=transformedSectionInput(input,desired);
  assert.equal(custom.basis,'custom');assert.equal(custom.face_id,null);assert.equal(custom.offset_mm,0);assert.deepEqual(custom.solid_ids,[0,2]);assert.deepEqual(localSectionFrame(custom,null)?.origin_mm,[10,10,37]);assert.equal(JSON.parse(JSON.stringify(custom)).custom_frame.origin_mm[2],37);
  custom.custom_frame!.origin_mm[2]=999;assert.equal(desired.origin_mm[2],37);assert.equal(input.offset_mm,30);
});
test('precise signed offset follows a rotated normal and presets reset orientation',()=>{
  const custom=transformedSectionInput(input,sectionFrameFromPose(new Vector3(3,4,5),new Quaternion().setFromAxisAngle(new Vector3(1,0,0),Math.PI/2))!);custom.offset_mm=-2;const actual=localSectionFrame(custom,null)!;near(actual.origin_mm[0],3);near(actual.origin_mm[1],6);near(actual.origin_mm[2],5);
  const xy=localSectionFrame({...custom,basis:'xy',custom_frame:null},null)!;assert.deepEqual(xy.normal,[0,0,1]);assert.deepEqual(xy.origin_mm,[0,0,-2]);const xz=localSectionFrame({...input,basis:'xz',offset_mm:4},null)!;assert.deepEqual(xz.origin_mm,[0,-4,0]);const yz=localSectionFrame({...input,basis:'yz',offset_mm:4},null)!;assert.deepEqual(yz.origin_mm,[4,0,0]);
});
test('captured pre-drag inputs and frame axes cannot change during a later gesture',()=>{
  const source=transformedSectionInput(input,frame),captured=cloneSectionInput(source);source.custom_frame!.origin_mm[0]=999;source.custom_frame!.normal[0]=0.5;source.solid_ids.push(9);assert.equal(captured.custom_frame!.origin_mm[0],100);assert.deepEqual(captured.custom_frame!.normal,[0,0,1]);assert.deepEqual(captured.solid_ids,[0,2]);
  assert.deepEqual(offsetSectionFrame(frame,2).origin_mm,[100,200,32]);assert.equal(frame.origin_mm[2],30);
});
test('invalid transform poses are refused before producing a custom request',()=>{
  assert.equal(sectionFrameFromPose(new Vector3(NaN,0,0),new Quaternion()),null);assert.equal(sectionFrameFromPose(new Vector3(1e9+1,0,0),new Quaternion()),null);assert.equal(sectionFrameFromPose(new Vector3(),new Quaternion(0,0,0,0)),null);assert.equal(localSectionFrame({...input,basis:'custom'},null),null);assert.equal(localSectionFrame({...input,basis:'face',face_id:1},null),null);
});
test('claimed gizmo pointer blocks picks before movement, during drag and until release or cancellation',()=>{
  const gesture=new SectionManipulationGesture();assert.equal(gesture.blocksPicking(),false);gesture.claim(7);assert.equal(gesture.blocksPicking(),true);gesture.begin();assert.equal(gesture.dragging,true);assert.equal(gesture.pointer,7);gesture.finish();assert.equal(gesture.blocksPicking(),false);assert.equal(gesture.dragging,false);assert.equal(gesture.pointer,null);gesture.finish();assert.equal(gesture.blocksPicking(),false);
});
test('installed TransformControls exposes only the normal arrow or two tilt rings and resets a drag',()=>{
  const dom={style:{touchAction:''},addEventListener(){},removeEventListener(){}} as unknown as HTMLElement;
  const camera=new PerspectiveCamera(40,1,0.01,100);camera.position.set(3,-4,4);camera.lookAt(0,0,0);camera.updateMatrixWorld();
  const control=new TransformControls(camera,dom),scene=new Scene(),anchor=new Object3D();scene.add(anchor);scene.add(control.getHelper());control.attach(anchor);
  configureSectionManipulator(control,'translate');assert.equal(control.space,'local');assert.equal(control.showX,false);assert.equal(control.showY,false);assert.equal(control.showZ,true);assert.equal(control.showXY,false);assert.equal(control.showE,false);
  configureSectionManipulator(control,'rotate');assert.equal(control.showX,true);assert.equal(control.showY,true);assert.equal(control.showZ,false);assert.equal(control.showXYZE,false);
  configureSectionManipulator(control,'translate');control.axis='Z';scene.updateMatrixWorld(true);let started=0,ended=0;control.addEventListener('mouseDown',()=>started++);control.addEventListener('mouseUp',()=>ended++);
  control.pointerDown({x:0,y:0,button:0} as unknown as PointerEvent);assert.equal(control.dragging,true);assert.equal(started,1);anchor.position.z=5;control.reset();assert.equal(anchor.position.z,0);control.pointerUp(null);assert.equal(control.dragging,false);assert.equal(control.axis,null);assert.equal(ended,1);control.dispose();assert.equal(dom.style.touchAction,'');
});
test('fresh capture hit handles a first click without hover, clears stale hover and ignores secondary pointers',()=>{
  const dom={style:{touchAction:''},addEventListener(){},removeEventListener(){}} as unknown as HTMLElement;
  const camera=new PerspectiveCamera(40,1,0.01,100);camera.position.set(3,-4,4);camera.lookAt(0,0,0);camera.updateMatrixWorld();
  const control=new TransformControls(camera,dom),scene=new Scene(),anchor=new Object3D();scene.add(anchor);scene.add(control.getHelper());control.attach(anchor);configureSectionManipulator(control,'translate');scene.updateMatrixWorld(true);
  const rect={left:100,top:50,width:800,height:600},gesture=new SectionManipulationGesture();let hit:{x:number;y:number}|null=null;
  for(let x=-0.4;x<=0.4&&!hit;x+=0.02)for(let y=-0.4;y<=0.4&&!hit;y+=0.02){control.pointerHover({x,y,button:0} as unknown as PointerEvent);if(control.axis==='Z')hit={x,y};}
  assert.ok(hit,'The installed gizmo normal arrow has a pickable location.');control.axis=null;
  const event={pointerId:17,isPrimary:true,clientX:rect.left+(hit.x+1)*rect.width/2,clientY:rect.top+(1-hit.y)*rect.height/2,button:0};
  assert.equal(claimSectionManipulator(control,gesture,event,rect),'claimed');assert.equal(control.axis,'Z');assert.equal(gesture.pointer,17);assert.equal(gesture.blocksPicking(),true);
  assert.equal(claimSectionManipulator(control,gesture,{...event,pointerId:18,isPrimary:false},rect),'ignored');assert.equal(gesture.pointer,17);gesture.finish();
  control.axis='Z';assert.equal(claimSectionManipulator(control,gesture,{...event,clientX:rect.left+rect.width,clientY:rect.top},rect),'orbit');assert.equal(control.axis,null);assert.equal(gesture.blocksPicking(),false);
  assert.deepEqual(sectionControlPointer({clientX:500,clientY:350,button:0},rect),{x:0,y:0,button:0});control.dispose();
});
