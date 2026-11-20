import {Matrix4,Quaternion,Vector3} from 'three';
import type {TransformControls} from 'three/examples/jsm/controls/TransformControls.js';
import type {ProfileFrame} from './types.ts';
import {sectionPlaneOverlay} from './section-plane.ts';
export type SectionManipulationMode='translate'|'rotate';
export function sectionControlPointer(event:{clientX:number;clientY:number;button:number},rect:{left:number;top:number;width:number;height:number}){
  return rect.width>0&&rect.height>0?{x:(event.clientX-rect.left)/rect.width*2-1,y:-(event.clientY-rect.top)/rect.height*2+1,button:event.button}:null;
}
export function configureSectionManipulator(control:TransformControls,mode:SectionManipulationMode){
  control.setSpace('local');control.setMode(mode);control.showX=mode==='rotate';control.showY=mode==='rotate';control.showZ=mode==='translate';control.showXY=false;control.showYZ=false;control.showXZ=false;control.showE=false;control.showXYZE=false;
}
/** A change of origin within the plane preserves its locus and gives rotation a nearby pivot. */
export function sectionManipulationPose(frame:ProfileFrame,bounds:number[][]) {
  const display=sectionPlaneOverlay(frame,bounds);
  if(!display)return null;
  const rotation=new Matrix4().makeBasis(new Vector3(...frame.x_axis),new Vector3(...frame.y_axis),new Vector3(...frame.normal));
  return {position:new Vector3(...display.center as [number,number,number]),quaternion:new Quaternion().setFromRotationMatrix(rotation).normalize()};
}
export function sectionFrameFromPose(position:Vector3,rotation:Quaternion):ProfileFrame|null {
  if([...position.toArray(),...rotation.toArray()].some(value=>!Number.isFinite(value))||position.toArray().some(value=>Math.abs(value)>1e9)||rotation.lengthSq()<1e-12)return null;
  const quaternion=rotation.clone().normalize();
  const x=new Vector3(1,0,0).applyQuaternion(quaternion),y=new Vector3(0,1,0).applyQuaternion(quaternion),normal=new Vector3().crossVectors(x,y).normalize();
  return {origin_mm:position.toArray(),x_axis:x.toArray(),y_axis:y.toArray(),normal:normal.toArray()};
}
/** A claimed gizmo pointer never becomes an orbit or a face click, including its release. */
export class SectionManipulationGesture {
  pointer:number|null=null;
  dragging=false;
  claim(pointer:number){if(this.pointer!==null&&this.pointer!==pointer)return false;this.pointer=pointer;return true;}
  begin(){this.dragging=true;}
  blocksPicking(){return this.pointer!==null||this.dragging;}
  finish(){this.pointer=null;this.dragging=false;}
}
/** Capture-phase hit resolution runs before OrbitControls gets the same pointerdown. */
export function claimSectionManipulator(control:TransformControls,gesture:SectionManipulationGesture,event:{pointerId:number;isPrimary?:boolean;clientX:number;clientY:number;button:number},rect:{left:number;top:number;width:number;height:number}):'ignored'|'claimed'|'orbit'{
  if(event.isPrimary===false||(gesture.pointer!==null&&gesture.pointer!==event.pointerId))return 'ignored';
  if(event.button!==0)return 'orbit';
  const pointer=sectionControlPointer(event,rect);if(!pointer)return 'orbit';
  control.getHelper().updateMatrixWorld(true);control.pointerHover(pointer as unknown as PointerEvent);
  if(control.axis===null)return 'orbit';
  gesture.claim(event.pointerId);return 'claimed';
}
