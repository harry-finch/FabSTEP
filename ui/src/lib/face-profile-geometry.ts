import type { FaceProfileSegment, PlanarFaceProfile, SectionProfile } from './types.ts';

const TAU = 2 * Math.PI;
export const PROFILE_PREVIEW_MAX_LOOPS = 1024, PROFILE_PREVIEW_MAX_SEGMENTS = 16384;
const failure = () => ({ code:'dxf.invalid_profile', message:'Invalid native planar profile.' });
/** SVG curves and analytic extrema come only from native edges, never display triangles. */
export function faceProfileGeometry(profile: Pick<PlanarFaceProfile,'frame'|'loops'>) {
  const finite = (values:number[]) => values.every(Number.isFinite);
  if (profile.loops.length > PROFILE_PREVIEW_MAX_LOOPS) throw { code:'dxf.too_complex', message:'Native planar profile exceeds display limits.' };
  if (!profile.loops.length || Object.values(profile.frame).some(v => v.length !== 3 || !finite(v))) throw failure();
  let count = 0, minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
  const include = (p:number[]) => {
    if (p.length !== 2 || !finite(p)) throw failure();
    minX = Math.min(minX,p[0]); minY = Math.min(minY,p[1]); maxX = Math.max(maxX,p[0]); maxY = Math.max(maxY,p[1]);
  };
  const point = (c:number[], r:number, a:number):[number,number] => [c[0]+r*Math.cos(a),c[1]+r*Math.sin(a)];
  // Same remainder convention as the native writer, including very large finite starts.
  const positive = (a:number) => {const remainder=a%TAU;return remainder<0?remainder+TAU:remainder;};
  function ellipse(segment:Extract<FaceProfileSegment,{kind:'ellipse'}>):string {
    const c=segment.center_mm,u=segment.major_axis_mm,ratio=segment.ratio,sweep=segment.sweep_param_rad;
    const validPoint=(point:unknown):point is [number,number]=>Array.isArray(point)&&point.length===2&&finite(point);
    if(!validPoint(c)||!validPoint(u)||!finite([ratio,segment.start_param_rad,sweep])||ratio<=0||ratio>1||sweep===0||Math.abs(sweep)>TAU)throw failure();
    const major=Math.hypot(...u),minor=major*ratio,v:[number,number]=[-u[1]*ratio,u[0]*ratio];
    if(!finite([major,minor,...v])||major<=0||minor<=0)throw failure();
    const xRadius=Math.hypot(u[0],v[0]),yRadius=Math.hypot(u[1],v[1]);
    if(!finite([c[0]-xRadius,c[0]+xRadius,c[1]-yRadius,c[1]+yRadius]))throw failure();
    const startParam=positive(Math.atan2(Math.sin(segment.start_param_rad),Math.cos(segment.start_param_rad))),rotation=Math.atan2(u[1],u[0])*180/Math.PI;
    const vertex=(parameter:number):[number,number]=>[c[0]+u[0]*Math.cos(parameter)+v[0]*Math.sin(parameter),c[1]+u[1]*Math.cos(parameter)+v[1]*Math.sin(parameter)];
    const start=vertex(startParam),end=vertex(startParam+sweep);include(start);include(end);
    // Each coordinate has two stationary parameters; only extrema within the signed arc belong to its bounds.
    for(const root of [Math.atan2(v[0],u[0]),Math.atan2(v[1],u[1])])for(const parameter of [root,root+Math.PI]){
      if(positive(sweep>0?parameter-startParam:startParam-parameter)<=Math.abs(sweep))include(vertex(parameter));
    }
    const direction=sweep>0?1:0,arc=`A ${major} ${minor} ${rotation}`;
    if(Math.abs(sweep)===TAU){const middle=vertex(startParam+sweep/2);return `M ${start.join(' ')} ${arc} 0 ${direction} ${middle.join(' ')} ${arc} 0 ${direction} ${start.join(' ')}`;}
    return `M ${start.join(' ')} ${arc} ${Math.abs(sweep)>Math.PI?1:0} ${direction} ${end.join(' ')}`;
  }
  function curve(segment:FaceProfileSegment):string {
    if (++count > PROFILE_PREVIEW_MAX_SEGMENTS) throw { code:'dxf.too_complex', message:'Native planar profile exceeds display limits.' };
    if (segment.kind === 'line') { include(segment.start_mm); include(segment.end_mm); return `M ${segment.start_mm.join(' ')} L ${segment.end_mm.join(' ')}`; }
    if(segment.kind==='ellipse')return ellipse(segment);
    const c = segment.center_mm, r = segment.radius_mm;
    if (!finite(c) || c.length !== 2 || !Number.isFinite(r) || r <= 0) throw failure();
    if (segment.kind === 'circle') {
      include([c[0]-r,c[1]-r]); include([c[0]+r,c[1]+r]);
      return `M ${c[0]+r} ${c[1]} A ${r} ${r} 0 1 1 ${c[0]-r} ${c[1]} A ${r} ${r} 0 1 1 ${c[0]+r} ${c[1]}`;
    }
    const a = segment.start_angle_rad, sweep = segment.sweep_angle_rad;
    if (!finite([a,sweep]) || Math.abs(sweep) <= 0 || Math.abs(sweep) > TAU + 1e-12) throw failure();
    const start = point(c,r,a), end = point(c,r,a+sweep);
    include(start); include(end);
    for (const cardinal of [0,Math.PI/2,Math.PI,3*Math.PI/2]) {
      const distance = positive(sweep > 0 ? cardinal-a : a-cardinal);
      if (distance <= Math.abs(sweep) + 1e-12) include(point(c,r,cardinal));
    }
    const direction = sweep > 0 ? 1 : 0;
    // Two analytic arcs preserve a possible full turn; SVG cannot encode it in one arc.
    if (Math.abs(sweep) >= TAU - 1e-12) {
      const mid = point(c,r,a+sweep/2);
      return `M ${start.join(' ')} A ${r} ${r} 0 1 ${direction} ${mid.join(' ')} A ${r} ${r} 0 1 ${direction} ${end.join(' ')}`;
    }
    return `M ${start.join(' ')} A ${r} ${r} 0 ${Math.abs(sweep)>Math.PI ? 1 : 0} ${direction} ${end.join(' ')}`;
  }
  const paths = profile.loops.map(loop => {
    if (!loop.segments.length) throw failure();
    return { outer:loop.is_outer, d:loop.segments.map(curve).join(' ') };
  });
  const width = maxX-minX, height = maxY-minY;
  if (!finite([width,height]) || width <= 0 || height <= 0) throw failure();
  const margin = Math.max(width,height)*0.08;
  const viewport=[minX-margin,-maxY-margin,width+2*margin,height+2*margin];
  if(!finite(viewport))throw failure();
  return { paths, width, height, bounds:[[minX,minY],[maxX,maxY]], viewBox:viewport.join(' ') };
}
export type FaceProfileGeometry = Omit<ReturnType<typeof faceProfileGeometry>,'paths'> & {paths:(ReturnType<typeof faceProfileGeometry>['paths'][number]&{solidId?:number;componentId?:number})[]};

/** Keep native body/component identities and a common frame; empty selected bodies stay explicit. */
export function sectionProfileGeometry(profile:SectionProfile):FaceProfileGeometry|null {
  if (!profile.bodies.length || profile.bodies.length > 32) throw failure();
  const loops = profile.bodies.flatMap(body=>body.loops);
  if (loops.length > PROFILE_PREVIEW_MAX_LOOPS) throw {code:'dxf.too_complex',message:'Native section exceeds display limits.'};
  if (!loops.length) return null;
  for(const body of profile.bodies){
    const components=new Map<number,{outer:number}>();
    for(const loop of body.loops){
      if(!Number.isInteger(loop.component_id)||loop.component_id<0)throw failure();
      const component=components.get(loop.component_id)??{outer:0};if(loop.is_outer)component.outer++;components.set(loop.component_id,component);
    }
    if([...components.values()].some(component=>component.outer!==1))throw failure();
  }
  const geometry = faceProfileGeometry({frame:profile.frame,loops});
  const identities = profile.bodies.flatMap(body=>body.loops.map(loop=>({solidId:body.solid_id,componentId:loop.component_id})));
  return { ...geometry, paths:geometry.paths.map((path,index)=>({...path,...identities[index]})) };
}
