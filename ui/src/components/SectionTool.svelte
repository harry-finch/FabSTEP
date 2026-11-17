<script lang="ts">
  // Section du STEP : réglages contextuels, solides explicites et aperçu fermé sans découper le document.
  import {Download,Move,RotateCw,RotateCcw,X} from '@lucide/svelte';
  import Section from './Section.svelte';
  import Disclosure from './Disclosure.svelte';
  import NumField from './NumField.svelte';
  import ProfilePreview from './ProfilePreview.svelte';
  import {i18n,t,type Key} from '../lib/i18n.svelte';
  import type {Diagnostic,SectionInput} from '../lib/types';
  import type {SourceSectionPreparationState} from '../lib/source-section';
  import type {SectionState} from '../lib/section';
  import type {SectionManipulationMode} from '../lib/section-manipulation';
  let {input,faceId,solids,state,canexport,onchange,onclose,onexport,errorText,mode,manipulating,onmodechange,onreset,target,sourcePreparation,bodyLabel,diagnosticLabel}:{input:SectionInput;faceId:number|null;solids:number[];state:SectionState;canexport:boolean;onchange:(input:SectionInput)=>void;onclose:()=>void;onexport:()=>void;errorText:(error:unknown)=>string;mode:SectionManipulationMode;manipulating:boolean;onmodechange:(mode:SectionManipulationMode)=>void;onreset:()=>void;target:'step'|'source';sourcePreparation:SourceSectionPreparationState;bodyLabel:(id:number)=>string;diagnosticLabel:(diagnostic:Diagnostic)=>string}=$props();
  const source=$derived(target==='source'),metadata=$derived(source?sourcePreparation.response?.metadata:null);
  const previewKeys=$derived({previewKey:(source?'sourceSection.preview':'section.preview') as Key,frameKey:(source?'sourceSection.frame':'faceProfile.frame') as Key,mappingKey:(source?'sourceSection.mapping':'faceProfile.mapping') as Key});
  const errors:Record<string,Key>={'dxf.section_coplanar':'section.coplanar','dxf.section_tangent':'section.tangent','dxf.section_ambiguous':'sourceSection.ambiguous','dxf.section_open':'section.open','dxf.section_empty':'section.empty','dxf.section_invalid_plane':'section.invalidPlane','dxf.unsupported_curve':'section.unsupported','dxf.too_complex':'faceProfile.tooComplex','dxf.invalid_profile':'faceProfile.invalidProfile','dxf.profile_not_ready':'section.pending','engine.stale':'faceProfile.stale','io.extension':'faceProfile.extension'};
  const error=$derived.by(()=>{void i18n.locale;if(!state.error)return '';const code=(state.error as {code?:string}).code;const key=source&&code==='dxf.section_coplanar'?'sourceSection.coplanar':source&&code==='dxf.section_tangent'?'sourceSection.tangent':code?errors[code]:undefined;return key?`${t(key)} (${code})`:errorText(state.error);});
  function basis(value:string){const next=value as SectionInput['basis'];onchange({...input,basis:next,face_id:next==='face'?faceId:null,custom_frame:null,offset_mm:next==='face'?0:input.offset_mm});}
  function body(id:number,checked:boolean){onchange({...input,solid_ids:checked?[...new Set([...input.solid_ids,id])].sort((a,b)=>a-b):input.solid_ids.filter(value=>value!==id)});}
  const holeCount=$derived(state.response?.profile.bodies.reduce((sum,body)=>sum+body.loops.filter(loop=>!loop.is_outer).length,0)??0);
</script>
<Section id={source?'source-section':'step-section'} title={t(source?'sourceSection.title':'section.title')}>
  {#snippet actions()}<button class="btn quiet icon" onclick={onclose} title={t('section.close')} aria-label={t('section.close')}><X size={14}/></button>{/snippet}
  <p class="muted">{t(source?'sourceSection.scope':'section.scope')}</p>
  {#if source&&sourcePreparation.pending}<p class="muted" role="status">{t('sourceSection.preparing')}</p>{/if}
  {#if source&&sourcePreparation.error}<p class="error" role="alert">{errorText(sourcePreparation.error)}</p>{/if}
  {#if !source||metadata}
  <div class="modes" role="group" aria-label={t('section.manipulation')}>
    <button class="btn" class:active={mode==='translate'} aria-pressed={mode==='translate'} disabled={manipulating} onclick={()=>onmodechange('translate')} title={t('section.moveHint')}><Move size={14}/>{t('section.move')}</button>
    <button class="btn" class:active={mode==='rotate'} aria-pressed={mode==='rotate'} disabled={manipulating} onclick={()=>onmodechange('rotate')} title={t('section.rotateHint')}><RotateCw size={14}/>{t('section.rotate')}</button>
    <button class="btn quiet icon" disabled={manipulating} onclick={onreset} title={t(source?'sourceSection.reset':'section.reset')} aria-label={t(source?'sourceSection.reset':'section.reset')}><RotateCcw size={14}/></button>
  </div>
  <p class="muted">{t(manipulating?'section.dragging':mode==='translate'?'section.moveHint':'section.rotateHint')}</p>
  <label class="basis"><span>{t('section.basis')}</span><select class="input" disabled={manipulating} value={input.basis} onchange={event=>basis(event.currentTarget.value)} title={t(source?'sourceSection.basisHint':'section.basisHint')}>
    <option value="xy">XY</option><option value="xz">XZ</option><option value="yz">YZ</option><option value="face" disabled={source||faceId===null}>{t('section.faceBasis')}</option>
    {#if input.basis==='custom'}<option value="custom" disabled>{t('section.custom')}</option>{/if}
  </select></label>
  {#if input.basis==='face'&&input.face_id!==null}<p class="muted">{t('section.faceNumber',{n:input.face_id+1})}</p>{/if}
  {#if input.basis==='face'&&input.face_id===null}<p class="error" role="alert">{t('section.selectFace')}</p>{/if}
  <NumField label={t('section.offset')} value={input.offset_mm} kind="plain" unit="mm" decimals={6} step={0.1} hint={t('section.offsetHint')} disabled={manipulating} invalid={!Number.isFinite(input.offset_mm)||Math.abs(input.offset_mm)>1e9} onchange={value=>onchange({...input,offset_mm:value})}/>
  <Disclosure id="dxf2-solids" label={t(source?'sourceSection.solids':'section.solids',{selected:input.solid_ids.length,total:solids.length})}>
    {#each solids as id}<label class="body"><input type="checkbox" disabled={manipulating} checked={input.solid_ids.includes(id)} onchange={event=>body(id,event.currentTarget.checked)}/><span>{source?bodyLabel(id):t('section.solid',{n:id+1})}</span></label>{/each}
  </Disclosure>
  {#if !input.solid_ids.length}<p class="error" role="alert">{t(source?'sourceSection.selectBody':'section.selectSolid')}</p>{/if}
  {#if state.pending}<p class="muted" role="status">{t(source?'sourceSection.pending':'section.pending')}</p>{/if}
  {#if state.geometry&&state.response}
    <ProfilePreview geometry={state.geometry} frame={state.response.profile.frame} holes={holeCount} id="dxf2-frame" {...previewKeys}/>
  {:else}
    {#if state.response}<p class="muted" role="status">{t('section.empty')}</p>{/if}
    {#if state.frame}<ProfilePreview geometry={null} frame={state.frame} holes={0} id="dxf2-frame" {...previewKeys}/>{/if}
  {/if}
  {#if state.response}
    <Disclosure id="dxf2-body-results" label={t('section.bodyResults')}>
      {#each state.response.profile.bodies as body}
        <div class="metric"><span>{source?bodyLabel(body.solid_id):t('section.solid',{n:body.solid_id+1})}</span><strong>{body.loops.length?t('section.components',{n:new Set(body.loops.map(loop=>loop.component_id)).size}):t('section.noIntersection')}</strong></div>
      {/each}
    </Disclosure>
  {/if}
  {/if}
  {#if metadata}
    <Disclosure id="dxf2b-preprocessing" label={t('sourceSection.preprocessing')}>
      <div class="metric"><span>{t('workspace.weld')}</span><strong>{metadata.settings.weld_tolerance_mm} mm</strong></div>
      {#each metadata.bodies as body}<p class="muted">{bodyLabel(body.solid_id)} · {t('sourceSection.counts',{input:body.input_triangles,triangles:body.triangles,welded:body.welded_vertices})}</p>{/each}
      {#each metadata.diagnostics as diagnostic}<p class="muted">{diagnosticLabel(diagnostic)} ({diagnostic.count}) <code>{diagnostic.code}</code></p>{/each}
    </Disclosure>
  {/if}
  {#if error}<p class="error" role="alert">{error}</p>{/if}
  <button class="btn" disabled={!canexport} onclick={onexport} title={t('section.exportHint')}><Download size={14}/>{t(state.exporting?'section.exporting':'section.export')}</button>
  {#if state.note}<p class="muted" role="status">{t(state.note==='saved'?'section.saved':'workspace.cancelled')}</p>{/if}
</Section>
<style>
  .basis{display:grid;grid-template-columns:100px minmax(0,1fr);gap:var(--s2);align-items:center;font-size:var(--fs-sm);color:var(--text-secondary)}
  .basis select{width:100%}.body{display:flex;align-items:center;gap:var(--s2);min-height:var(--control-h);font-size:var(--fs-sm);color:var(--text-secondary)}
  .modes{display:flex;gap:var(--s1)}.modes .btn:not(.icon){flex:1;padding-inline:var(--s2)}.modes .active{background:var(--bg-selected);border-color:var(--accent);color:var(--accent-text)}
  .body input{accent-color:var(--accent)}.metric{display:flex;justify-content:space-between;align-items:center;gap:var(--s2);min-height:var(--control-h);font-size:var(--fs-xs);color:var(--text-secondary)}
  .metric strong{font-weight:500;text-align:right;color:var(--text-primary)}
  .body span,.metric span{overflow-wrap:anywhere}
  .muted,.error{margin:0;font-size:var(--fs-xs);line-height:1.5;color:var(--text-muted)}.error{color:var(--danger-text);overflow-wrap:anywhere}
</style>
