import type {ProfileFrame,SectionInput} from './types.ts';

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
export function transformedSectionInput(input:SectionInput,frame:ProfileFrame):SectionInput {
  return {...input,basis:'custom',face_id:null,offset_mm:0,custom_frame:cloneSectionFrame(frame),solid_ids:[...input.solid_ids]};
}
