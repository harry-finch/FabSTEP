import type {ProfileFrame} from './types.ts';
/** Eight bound corners size a display rectangle; this never computes or exports a section. */
export function sectionPlaneOverlay(frame:ProfileFrame,bounds:number[][]) {
  if(bounds.length!==2||bounds.some(point=>point.length!==3||point.some(value=>!Number.isFinite(value))))return null;
  const dot=(a:number[],b:number[])=>a.reduce((sum,value,index)=>sum+value*b[index],0);
  let minX=Infinity,minY=Infinity,maxX=-Infinity,maxY=-Infinity;
  for(let x=0;x<2;x++)for(let y=0;y<2;y++)for(let z=0;z<2;z++){
    const delta=[bounds[x][0],bounds[y][1],bounds[z][2]].map((value,index)=>value-frame.origin_mm[index]);
    const u=dot(delta,frame.x_axis),v=dot(delta,frame.y_axis);minX=Math.min(minX,u);maxX=Math.max(maxX,u);minY=Math.min(minY,v);maxY=Math.max(maxY,v);
  }
  const span=Math.max(maxX-minX,maxY-minY,0.01),margin=span*0.1;
  const world=(u:number,v:number)=>frame.origin_mm.map((value,index)=>value+u*frame.x_axis[index]+v*frame.y_axis[index]);
  const corners=[world(minX-margin,minY-margin),world(maxX+margin,minY-margin),world(maxX+margin,maxY+margin),world(minX-margin,maxY+margin)];
  return {positions:[...corners[0],...corners[1],...corners[2],...corners[0],...corners[2],...corners[3]],outline:corners.flatMap((corner,index)=>[...corner,...corners[(index+1)%4]]),center:world((minX+maxX)/2,(minY+maxY)/2),normal:[...frame.normal],arrowLength:span*0.2};
}
