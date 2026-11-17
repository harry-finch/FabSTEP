import type { ResultFaces, StepFamily } from './types.ts';
export type StepSourceSelection = { status:'exact'; bodies:{ previewBodyId:string; triangles:number[] }[]; triangleCount:number } | { status:'unavailable'; reason:string };
export type StepSelection = { faces:number[]; triangles:number[]; families:Partial<Record<StepFamily,number>>; solids:number[]; hasMore:boolean; canShrink:boolean; source:StepSourceSelection };
const families = new Set(['plane','cylinder','cone','sphere','torus','other']);
const MAX_TRIANGLES = 100_000, MAX_VERTICES = 300_000, MAX_INCIDENCES = 600_000;
const invalid = () => new Error('step.invalid_preview');
/** Sélection des vraies faces du STEP ; validation et croissance bornées dans un worker. */
export class StepSelectionGraph {
  private document:ResultFaces|null = null;
  private selected = new Set<number>();
  private history:number[][] = [];
  private sourceFaces = new Map<number,{ previewBodyId:string; triangles:number[] }>();
  private sourceReason = 'step.provenance_unavailable';
  prepare(document:ResultFaces, bodyCounts:Record<string,number> = {}) {
    this.document = null; this.selected.clear(); this.history = [];
    const { mesh, faces } = document;
    if (!mesh.triangles.length || mesh.triangles.length > MAX_TRIANGLES || !mesh.vertices.length || mesh.vertices.length > MAX_VERTICES || !faces.length || faces.length > MAX_TRIANGLES || !Number.isFinite(document.deflection_mm) || document.deflection_mm <= 0) throw invalid();
    for (const vertex of mesh.vertices) if (vertex.length !== 3 || vertex.some(n => !Number.isFinite(n))) throw invalid();
    for (const triangle of mesh.triangles) if (triangle.length !== 3 || new Set(triangle).size !== 3 || triangle.some(n => !Number.isInteger(n) || n < 0 || n >= mesh.vertices.length)) throw invalid();
    let at = 0, incidences = 0;
    const neighbors:Set<number>[] = [];
    for (const [id,face] of faces.entries()) {
      if (face.id !== id || !Number.isInteger(face.solid_id) || face.solid_id < 0 || face.solid_id >= 32 || !families.has(face.family) || face.triangle_start !== at || !Number.isInteger(face.triangle_count) || face.triangle_count <= 0 || face.triangle_count > MAX_TRIANGLES || (at += face.triangle_count) > mesh.triangles.length || (incidences += face.adjacent_face_ids.length) > MAX_INCIDENCES) throw invalid();
      const row = new Set(face.adjacent_face_ids);
      if (row.size !== face.adjacent_face_ids.length || [...row].some(n => !Number.isInteger(n) || n < 0 || n >= faces.length || n === id || faces[n].solid_id !== face.solid_id)) throw invalid();
      neighbors.push(row);
    }
    if (at !== mesh.triangles.length) throw invalid();
    for (const [id,row] of neighbors.entries()) for (const adjacent of row) if (!neighbors[adjacent].has(id)) throw invalid();
    this.document = document;
    this.prepareSource(document,bodyCounts);
  }
  /** Une provenance douteuse désactive seulement le lien source, jamais les faces STEP. */
  private prepareSource(document:ResultFaces, bodyCounts:Record<string,number>) {
    this.sourceFaces.clear(); this.sourceReason = 'step.provenance_invalid';
    try {
      const correspondence = document.source_correspondence;
      if (correspondence.status === 'unavailable') {
        if (typeof correspondence.reason === 'string' && correspondence.reason.length > 0 && correspondence.reason.length <= 128) this.sourceReason = correspondence.reason;
        return;
      }
      if (correspondence.status !== 'exact' || correspondence.faces.length !== document.faces.length) return;
      const counts = new Map(Object.entries(bodyCounts));
      if (!counts.size || counts.size > 32) return;
      let expected = 0, assigned = 0;
      for (const [id,count] of counts) if (!id || !Number.isInteger(count) || count <= 0 || (expected += count) > MAX_TRIANGLES) return;
      const used = new Map<string,Set<number>>(), analysisBodies = new Map<string,string>();
      for (const row of correspondence.faces) {
        const count = counts.get(row.preview_body_id);
        if (!Number.isInteger(row.face_id) || !document.faces[row.face_id] || this.sourceFaces.has(row.face_id) || typeof row.analysis_body_id !== 'string' || !row.analysis_body_id || count === undefined || !row.source_triangle_ids.length || (assigned += row.source_triangle_ids.length) > MAX_TRIANGLES) return;
        const previous = analysisBodies.get(row.analysis_body_id);
        if (previous !== undefined && previous !== row.preview_body_id) return;
        analysisBodies.set(row.analysis_body_id,row.preview_body_id);
        const seen = used.get(row.preview_body_id) ?? new Set<number>();
        let last = -1;
        for (const index of row.source_triangle_ids) {
          if (!Number.isInteger(index) || index <= last || index >= count || seen.has(index)) return;
          seen.add(index); last = index;
        }
        used.set(row.preview_body_id,seen);
        this.sourceFaces.set(row.face_id,{ previewBodyId:row.preview_body_id, triangles:row.source_triangle_ids });
      }
      if (assigned !== expected || [...counts].some(([id,count]) => used.get(id)?.size !== count)) return;
      this.sourceReason = '';
    } catch { /* Un DTO incomplet ne doit pas invalider le STEP courant. */ }
    finally { if (this.sourceReason) this.sourceFaces.clear(); }
  }
  pick(id:number, additive:boolean) {
    if (!this.document || !Number.isInteger(id) || !this.document.faces[id]) throw invalid();
    if (!additive) this.selected.clear();
    if (additive && this.selected.has(id)) this.selected.delete(id); else this.selected.add(id);
    this.history = []; return this.inspect();
  }
  select(ids:number[]) {
    if (!this.document || !ids.length || ids.length>this.document.faces.length || new Set(ids).size!==ids.length || ids.some(id=>!Number.isInteger(id)||!this.document!.faces[id])) throw invalid();
    const solid=this.document.faces[ids[0]].solid_id;
    if(ids.some(id=>this.document!.faces[id].solid_id!==solid))throw invalid();
    this.selected=new Set(ids);this.history=[];return this.inspect();
  }
  add(ids:number[]) {
    if(!this.document || ids.length>MAX_TRIANGLES || ids.some(id=>!Number.isInteger(id)||!this.document!.faces[id]))throw invalid();
    for(const id of ids)this.selected.add(id);
    this.history=[];return this.inspect();
  }
  clear() { this.selected.clear(); this.history = []; return this.inspect(); }
  grow() {
    if (!this.document) throw invalid();
    const next = new Set(this.selected);
    for (const id of this.selected) for (const adjacent of this.document.faces[id].adjacent_face_ids) next.add(adjacent);
    if (next.size > this.selected.size) { this.history.push([...next].filter(id => !this.selected.has(id))); this.selected = next; }
    return this.inspect();
  }
  shrink() { const added = this.history.pop(); if (added) for (const id of added) this.selected.delete(id); return this.inspect(); }
  inspect():StepSelection {
    if (!this.document) throw invalid();
    const selection:StepSelection = { faces:[...this.selected].sort((a,b) => a-b), triangles:[], families:{}, solids:[], hasMore:false, canShrink:!!this.history.length, source:{ status:'unavailable', reason:this.sourceReason } };
    const solids = new Set<number>();
    for (const id of selection.faces) {
      const face = this.document.faces[id]; solids.add(face.solid_id);
      selection.families[face.family] = (selection.families[face.family] ?? 0)+1;
      for (let i=face.triangle_start;i<face.triangle_start+face.triangle_count;i++) selection.triangles.push(i);
      if (face.adjacent_face_ids.some(n => !this.selected.has(n))) selection.hasMore = true;
    }
    if (!this.sourceReason) {
      const groups = new Map<string,number[]>();
      let triangleCount = 0;
      for (const id of selection.faces) {
        const source = this.sourceFaces.get(id)!;
        const triangles = groups.get(source.previewBodyId) ?? [];
        for (const triangle of source.triangles) triangles.push(triangle);
        triangleCount += source.triangles.length; groups.set(source.previewBodyId,triangles);
      }
      selection.source = { status:'exact', triangleCount, bodies:[...groups].map(([previewBodyId,triangles]) => ({ previewBodyId,triangles:triangles.sort((a,b) => a-b) })) };
    }
    selection.solids = [...solids].sort((a,b) => a-b); return selection;
  }
}
