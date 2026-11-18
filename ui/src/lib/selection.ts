import type { BodyResult, SelectionTopology } from './types.ts';

export type SourceZone = { previewBodyId:string; seed:number; triangles:number[]; ring:number };
export type SurfaceFamily = 'plane'|'cylinder'|'partial_cylinder'|'partial_cone'|'cone'|'sphere'|'spherical_octant'|'linear_blend'|'torus';
export type ZoneInspection = { families:Partial<Record<SurfaceFamily, number>>; unavailable:number; analysisBodyIds:string[] };
export type Attribution = { analysisBodyId:string; families:SurfaceFamily[] | null };
const MAX_TRIANGLES = 100_000;
const families = new Set(['plane','cylinder','partial_cylinder','partial_cone','cone','sphere','spherical_octant','linear_blend','torus']);

/** Un clic doit rester immobile, primaire et seul pendant tout le geste. */
export class SourceClickGesture {
  private pointers = new Set<number>();
  private candidate: { id:number; x:number; y:number } | null = null;
  down(e: { pointerId:number; clientX:number; clientY:number; button:number; isPrimary?:boolean; ctrlKey?:boolean; metaKey?:boolean; shiftKey?:boolean; altKey?:boolean }, allowModifiers = false) {
    this.pointers.add(e.pointerId);
    this.candidate = this.pointers.size === 1 && e.button === 0 && e.isPrimary !== false && !e.altKey && (allowModifiers || !this.modified(e)) ? { id:e.pointerId, x:e.clientX, y:e.clientY } : null;
  }
  move(e: { pointerId:number; clientX:number; clientY:number }) {
    const c = this.candidate;
    if (c && c.id === e.pointerId && Math.hypot(e.clientX-c.x, e.clientY-c.y) > 4) this.candidate = null;
  }
  up(e: { pointerId:number; clientX:number; clientY:number; button:number; isPrimary?:boolean; ctrlKey?:boolean; metaKey?:boolean; shiftKey?:boolean; altKey?:boolean }, allowModifiers = false): boolean {
    this.move(e);
    const click = this.pointers.size === 1 && this.candidate?.id === e.pointerId && e.button === 0 && !e.altKey && (allowModifiers || !this.modified(e));
    this.pointers.delete(e.pointerId); this.candidate = null;
    return !!click;
  }
  cancel() { this.pointers.clear(); this.candidate = null; }
  private modified(e: { ctrlKey?:boolean; metaKey?:boolean; shiftKey?:boolean; altKey?:boolean }) { return e.ctrlKey || e.metaKey || e.shiftKey || e.altKey; }
}

/** Garde indépendante : effacer une zone invalide aussi les réponses du worker. */
export class SelectionReplies {
  revision = 0; epoch = 0; job = 0;
  newSource() { this.revision++; this.clear(); }
  clear() { this.epoch++; this.job++; }
  next(requestId:number) { return { revision:this.revision, epoch:this.epoch, job:++this.job, requestId }; }
  accepts(token:SelectionToken, requestId:number) { return token.revision === this.revision && token.epoch === this.epoch && token.job === this.job && token.requestId === requestId; }
}
export type SelectionToken = { revision:number; epoch:number; job:number; requestId:number };

/** Pas d'inférence depuis les coordonnées : seules les attributions exhaustives comptent. */
export function attributions(results:BodyResult[]):Attribution[] {
  return results.map(body => {
    let entries:{kind:string; index?:number}[] | null = null;
    let bounds:Partial<Record<SurfaceFamily, number>> = {};
    if (body.rounded_box_recognition) {
      entries = body.rounded_box_recognition.triangle_faces;
    } else {
      const rounded = body.rounded_recognition, revolved = rounded?.model.revolved ?? body.revolved_recognition?.model;
      const mixed = revolved?.mixed ?? body.mixed_recognition?.model;
      entries = rounded?.triangle_faces ?? body.revolved_recognition?.triangle_faces ?? body.mixed_recognition?.triangle_faces ?? null;
      bounds = { plane:mixed?.planes.length, cylinder:mixed?.cylinders.length, partial_cylinder:mixed?.partial_cylinders.length, cone:revolved?.cones.length, partial_cone:rounded?.model.partial_cones?.length, sphere:rounded?.model.spheres.length, spherical_octant:rounded?.model.octants?.length, linear_blend:rounded?.model.linear_blend ? 1 : 0, torus:rounded?.model.tori.length };
    }
    const valid = entries && entries.length === body.analysis.mesh.triangles.length && entries.every(face => families.has(face.kind) && (body.rounded_box_recognition || (Number.isInteger(face.index) && face.index! >= 0 && face.index! < (bounds[face.kind as SurfaceFamily] ?? 0))));
    return { analysisBodyId:body.id, families:valid ? entries!.map(face => face.kind as SurfaceFamily) : null };
  });
}

type Component = { analysisId:string; source:number[]; sourceToLocal:Map<number,number>; neighbors:number[][] };
/** Graphe borné, calculé une fois par révision dans le worker. Aucun graphe géométrique. */
export class SelectionGraph {
  private bodies = new Map<string, Component[]>();
  private sources = new Map<string, Map<number, { component:Component; local:number }>>();
  private attribution = new Map<string, SurfaceFamily[] | null>();
  private rings: { key:string; distances:Map<number,number>; max:number } | null = null;
  prepare(topology:SelectionTopology, bodyCounts:Record<string,number>) {
    this.bodies.clear(); this.sources.clear(); this.attribution.clear(); this.rings = null;
    if (topology.components.length > 32 || Object.keys(bodyCounts).length > 32) throw new Error('selection.invalid_topology');
    let total = 0;
    const analysisIds = new Set<string>();
    for (const item of topology.components) {
      const count = bodyCounts[item.preview_body_id];
      const source = item.analysis_to_source_triangles;
      if (!Number.isInteger(count) || count < 0 || count > MAX_TRIANGLES || analysisIds.has(item.analysis_body_id) || source.length !== item.source_triangle_neighbors.length || (total += source.length) > MAX_TRIANGLES) throw new Error('selection.invalid_topology');
      analysisIds.add(item.analysis_body_id);
      const sourceToLocal = new Map<number,number>();
      source.forEach((id, index) => {
        if (!Number.isInteger(id) || id < 0 || id >= count || sourceToLocal.has(id)) throw new Error('selection.invalid_topology');
        sourceToLocal.set(id,index);
      });
      for (let i=0; i<source.length; i++) {
        const row = item.source_triangle_neighbors[i];
        if (row.length > 3 || new Set(row).size !== row.length || row.some(id => id === source[i] || !sourceToLocal.has(id))) throw new Error('selection.invalid_topology');
        for (const id of row) if (!item.source_triangle_neighbors[sourceToLocal.get(id)!].includes(source[i])) throw new Error('selection.invalid_topology');
      }
      const components = this.bodies.get(item.preview_body_id) ?? [];
      const component = { analysisId:item.analysis_body_id, source, sourceToLocal, neighbors:item.source_triangle_neighbors };
      const lookup = this.sources.get(item.preview_body_id) ?? new Map<number, { component:Component; local:number }>();
      source.forEach((id,local) => {
        if (lookup.has(id)) throw new Error('selection.invalid_topology');
        lookup.set(id,{ component,local });
      });
      components.push(component); this.bodies.set(item.preview_body_id,components); this.sources.set(item.preview_body_id,lookup);
    }
    for (const [id,count] of Object.entries(bodyCounts)) if ((this.bodies.get(id) ?? []).reduce((n,c) => n+c.source.length,0) !== count) throw new Error('selection.invalid_topology');
  }
  interpret(items:Attribution[]) { this.attribution = new Map(items.map(item => [item.analysisBodyId,item.families])); }
  inspect(bodyId:string, triangles:number[]):ZoneInspection {
    if (triangles.length > MAX_TRIANGLES) throw new Error('selection.invalid_zone');
    const counts:ZoneInspection = { families:{}, unavailable:0, analysisBodyIds:[] };
    const lookup = this.sources.get(bodyId);
    for (const id of triangles) {
      const match = lookup?.get(id);
      if (!match) { counts.unavailable++; continue; }
      const { component,local } = match;
      if (!counts.analysisBodyIds.includes(component.analysisId)) counts.analysisBodyIds.push(component.analysisId);
      const assigned = this.attribution.get(component.analysisId);
      const family = assigned?.length === component.source.length ? assigned[local] : null;
      if (family && families.has(family)) counts.families[family] = (counts.families[family] ?? 0)+1; else counts.unavailable++;
    }
    return counts;
  }
  zone(bodyId:string, seed:number, base:number[], ring:number, epoch:number) {
    if (!Number.isInteger(ring) || ring < 0 || ring > MAX_TRIANGLES || !base.length || base.length > MAX_TRIANGLES) throw new Error('selection.invalid_zone');
    const key = `${bodyId}\0${seed}\0${epoch}`;
    if (this.rings?.key !== key) {
      const lookup = this.sources.get(bodyId);
      if (!lookup?.has(seed) || base.some(id => !lookup.has(id))) throw new Error('selection.invalid_zone');
      const distances = new Map<number,number>(); const queue:number[] = [];
      for (const id of base) if (!distances.has(id)) { distances.set(id,0); queue.push(id); }
      let max = 0;
      for (let at=0; at<queue.length; at++) {
        const id = queue[at], distance = distances.get(id)!;
        const { component,local } = lookup.get(id)!;
        for (const next of component.neighbors[local]) if (!distances.has(next)) {
          distances.set(next,distance+1); queue.push(next); max = distance+1;
        }
      }
      this.rings = { key, distances, max };
    }
    const triangles = [...this.rings.distances].filter(([,distance]) => distance <= ring).map(([id]) => id).sort((a,b) => a-b);
    return { triangles, hasMore:ring < this.rings.max, inspection:this.inspect(bodyId,triangles) };
  }
}
