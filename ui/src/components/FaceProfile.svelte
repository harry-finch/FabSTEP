<script lang="ts">
  // Profil 2D contextuel : courbes natives, repère explicite, export du seul résultat courant.
  import Download from '@lucide/svelte/icons/download';
  import ProfilePreview from './ProfilePreview.svelte';
  import { i18n, t, type Key } from '../lib/i18n.svelte';
  import type { FaceProfileState } from '../lib/face-profile';
  let { state, canexport, onexport, errorText }: { state:FaceProfileState; canexport:boolean; onexport:()=>void; errorText:(error:unknown)=>string } = $props();
  const geometry = $derived(state.geometry);
  const errors:Record<string,Key> = { 'dxf.face_not_found':'faceProfile.faceNotFound', 'dxf.face_not_planar':'faceProfile.notPlanar', 'dxf.unsupported_curve':'faceProfile.unsupportedCurve', 'dxf.invalid_profile':'faceProfile.invalidProfile', 'dxf.too_complex':'faceProfile.tooComplex', 'dxf.profile_not_ready':'faceProfile.pending', 'engine.stale':'faceProfile.stale', 'io.extension':'faceProfile.extension' };
  const error = $derived.by(() => {
    void i18n.locale;
    if (!state.error) return '';
    const code = (state.error as {code?:string}).code;
    return code && errors[code] ? `${t(errors[code])} (${code})` : errorText(state.error);
  });
</script>
<div class="profile">
  <div class="label">{t('faceProfile.title')}</div>
  {#if state.pending}
    <p class="muted" role="status">{t('faceProfile.pending')}</p>
  {/if}
  {#if geometry && state.response}
    <ProfilePreview {geometry} frame={state.response.profile.frame} holes={state.response.profile.loops.filter(loop => !loop.is_outer).length}/>
  {/if}
  {#if error}<p class="error" role="alert">{error}</p>{/if}
  <button class="btn" disabled={!canexport} title={t('faceProfile.scope')} onclick={onexport}><Download size={14}/>{t(state.exporting ? 'faceProfile.exporting' : 'faceProfile.export')}</button>
  {#if state.note}<p class="muted" role="status">{t(state.note === 'saved' ? 'faceProfile.saved' : 'workspace.cancelled')}</p>{/if}
  <p class="muted">{t('faceProfile.scope')}</p>
</div>
<style>
  .profile{display:flex;flex-direction:column;gap:var(--s2);padding-top:var(--s2);border-top:1px solid var(--divider)}
  .label{font-size:var(--fs-xs);font-weight:600;color:var(--text-section)}
  .muted,.error{margin:0;font-size:var(--fs-xs);line-height:1.5;color:var(--text-muted)}.error{color:var(--danger-text);overflow-wrap:anywhere}
</style>
