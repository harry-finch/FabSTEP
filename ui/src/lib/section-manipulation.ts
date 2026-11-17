import {Matrix4,Quaternion,Vector3} from 'three';
import type {TransformControls} from 'three/examples/jsm/controls/TransformControls.js';
import type {ProfileFrame,SectionInput} from './types.ts';
import {sectionPlaneOverlay} from './section-plane.ts';
export type SectionManipulationMode='translate'|'rotate';
export function sectionControlPointer(event:{clientX:number;clientY:number;button:number},rect:{left:number;top:number;width:number;height:number}){
  return rect.width>0&&rect.height>0?{x:(event.clientX-rect.left)/rect.width*2-1,y:-(event.clientY-rect.top)/rect.height*2+1,button:event.button}:null;
}
export function configureSectionManipulator(control:TransformControls,mode:SectionManipulationMode){
  control.setSpace('local');control.setMode(mode);control.showX=mode==='rotate';control.showY=mode==='rotate';control.showZ=mode==='translate';control.showXY=false;control.showYZ=false;control.showXZ=false;control.showE=false;control.showXYZE=false;
}
export const cloneSectionFrame=(frame:ProfileFrame):ProfileFrame=>({origin_mm:[...frame.origin_mm],x_axis:[...frame.x_axis],y_axis:[...frame.y_axis],normal:[...frame.normal]});
export const cloneSectionInput=(input:SectionInput):SectionInput=>({...input,solid_ids:[...input.solid_ids],...(input.custom_frame?{custom_frame:cloneSectionFrame(input.custom_frame)}:{})});
export function offsetSectionFrame(frame:ProfileFrame,offset:number):ProfileFrame {
  const result=cloneSectionFrame(frame);result.origin_mm=result.origin_mm.map((value,index)=>value+offset*result.normal[index]) as [number,number,number];return result;
}
/** Presets are explicit local display coordinates; the native side independently validates them. */
export function localSectionFrame(input:SectionInput,faceBase:ProfileFrame|null):ProfileFrame|null {
  let frame:ProfileFrame|null=null;
  if(input.basis==='custom')frame=input.custom_frame?cloneSectionFrame(input.custom_frame):null;
  else if(input.basis==='face')frame=faceBase?cloneSectionFrame(faceBase):null;
  else{
    const axes={xy:[[1,0,0],[0,1,0],[0,0,1]],xz:[[1,0,0],[0,0,1],[0,-1,0]],yz:[[0,1,0],[0,0,1],[1,0,0]]}[input.basis];
    frame={origin_mm:[0,0,0],x_axis:[...axes[0]] as [number,number,number],y_axis:[...axes[1]] as [number,number,number],normal:[...axes[2]] as [number,number,number]};
  }
  return frame&&Number.isFinite(input.offset_mm)?offsetSectionFrame(frame,input.offset_mm):null;
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
export function transformedSectionInput(input:SectionInput,frame:ProfileFrame):SectionInput {
  return {...input,basis:'custom',face_id:null,offset_mm:0,custom_frame:cloneSectionFrame(frame),solid_ids:[...input.solid_ids]};
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
