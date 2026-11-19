<script lang="ts">
  import { onMount } from 'svelte';
  import CircleHelp from '@lucide/svelte/icons/circle-help';
  import Download from '@lucide/svelte/icons/download';
  import FileUp from '@lucide/svelte/icons/file-up';
  import Focus from '@lucide/svelte/icons/focus';
  import Monitor from '@lucide/svelte/icons/monitor';
  import Moon from '@lucide/svelte/icons/moon';
  import MousePointer2 from '@lucide/svelte/icons/mouse-pointer-2';
  import Scissors from '@lucide/svelte/icons/scissors';
  import Sun from '@lucide/svelte/icons/sun';
  import TriangleAlert from '@lucide/svelte/icons/triangle-alert';
  import ParameterSearch from './components/ParameterSearch.svelte';
  import { ParameterSearchReplies, proposedTolerance, type SearchToken, type ParameterSearchResponse } from './lib/parameter-search';
  import Section from './components/Section.svelte';
  import Disclosure from './components/Disclosure.svelte';
  import Viewport from './components/Viewport.svelte';
  import ProcessingIndicator from './components/ProcessingIndicator.svelte';
  import { watchConversionProgress, type ConversionProgress } from './lib/conversion-progress';
  import Dialog from './components/Dialog.svelte';
  import NumField from './components/NumField.svelte';
  import SourceInspection from './components/SourceInspection.svelte';
  import StepInspection from './components/StepInspection.svelte';
  import GuidedCorrectionBar from './components/GuidedCorrectionBar.svelte';
  import StepSourceInspection from './components/StepSourceInspection.svelte';
  import SectionTool from './components/SectionTool.svelte';
  import { restoreOptions } from './lib/options-storage';
  import { FacePaintQueue } from './lib/face-paint';
  import { RegionSelectionQueue } from './lib/region-selection';
  import { canPrepareCorrection, guidedFaces, guidedFamilyLabel, isGuidedRefusal } from './lib/guided';
  import { ContextReveal } from './lib/context-reveal';
  import { visibleDiagnostic, triangleLimitCounts } from './lib/diagnostics';
  import type { StepDisplay } from './lib/step-display';
  import type { StepSelection } from './lib/step-selection';
  import { SelectionReplies, attributions, type SelectionToken, type SourceZone, type ZoneInspection } from './lib/selection';
  import { validPreferences, sameOptions, commitSnapshot, bodyLabel, errorContext } from './lib/editing';
  import type { SourceBridge } from './lib/source';
  import { createWorkflow, type WorkflowState } from './lib/workflow';
  import { createFaceProfiles, type FaceProfileContext, type FaceProfileState } from './lib/face-profile';
  import { createFaceProfileGeometryWorker } from './lib/face-profile-worker';
  import type { PlanarFaceProfile, ProfileFrame, SectionInput, SectionProfile } from './lib/types';
  import {createSections,type SectionContext,type SectionState} from './lib/section';
  import {createSourceSectionPreparation,initialSourceSectionInput,type SourceSectionPreparationContext,type SourceSectionPreparationState} from './lib/source-section';
  import {cloneSectionFrame,cloneSectionInput,localSectionFrame,offsetSectionFrame,transformedSectionInput,type SectionManipulationMode} from './lib/section-manipulation';
  import packageInfo from '../package.json';
  import { theme } from './lib/theme.svelte';
  import { i18n, LOCALES, t, type Key } from './lib/i18n.svelte';
  import type { Options, Result, DocumentPreview, BodyResult, SelectionTopology, ResultFaces, GuidedFamily, GuidedReport } from './lib/types';
  const initial: Options = { input_unit:'mm', weld_tolerance_mm:0.000001, plane_tolerance_mm:0.000001, reconstruction:'auto', cylinder_fit_tolerance_mm:0.00001, max_deviation_mm:0.05 };
  let options = $state<Options>({ ...initial });
  const positiveOr = (value: unknown, fallback: number): number => typeof value === 'number' && Number.isFinite(value) && value > 0 ? value : fallback;
  function serializableOptions(value: Options): Options {
    return { ...value, weld_tolerance_mm:positiveOr(value.weld_tolerance_mm, initial.weld_tolerance_mm), plane_tolerance_mm:positiveOr(value.plane_tolerance_mm, initial.plane_tolerance_mm), cylinder_fit_tolerance_mm:positiveOr(value.cylinder_fit_tolerance_mm, initial.cylinder_fit_tolerance_mm), max_deviation_mm:positiveOr(value.max_deviation_mm, initial.max_deviation_mm) };
  }
  try { options = restoreOptions(initial,localStorage.getItem('fabstep.options.v2'),localStorage.getItem('fabstep.options.v1')); } catch { /* stockage optionnel */ }
  let bytes = $state.raw<Uint8Array | null>(null), name = $state(''), result = $state.raw<Result | null>(null), preview = $state.raw<DocumentPreview | null>(null);
  let errorState = $state<{ kind: 'key'; key: Key } | { kind: 'native'; value: unknown } | null>(null);
  const error = $derived.by(() => { void i18n.locale; return errorState ? (errorState.kind === 'key' ? t(errorState.key) : errorText(errorState.value)) : ''; });
  let format = $state<'stl'|'3mf'>('stl'), selectedId = $state('');
  function initialCommit(): Options { return { ...options }; }
  let committed: Options = initialCommit();
  let showHelp = $state(false);
  let noteKey = $state<Key | null>(null);
  let recognitionFailed = $state(false);
  let busy = $state(false), dragging = $state(false), showEdges = $state(true), fitSignal = $state(0);
  // Le verrou du moteur peut appartenir à une ancienne requête ; l’indicateur suit la demande courante.
  let processing = $state<{ requestId:number; phase:'queued'|'preview'|'converting'|'searching'; startedAt:number } | null>(null);
  let input: HTMLInputElement; let current = $state(Date.now() * 1000); let fileToken = 0, queued = false; let history: Options[] = [], future: Options[] = [];
  let sourceId: number | null = null, uploadedSourceId: number | null = null;
  // L'identité géométrique source reste indépendante des requêtes de conversion.
  let sourceRevision = $state(0), selectionMode = $state(false), sourceZone = $state.raw<SourceZone|null>(null);
  let zoneInspection = $state<ZoneInspection|null>(null), topologyState = $state<'pending'|'ready'|'unavailable'>('unavailable'), hasMore = $state(true);
  const selectionReplies = new SelectionReplies();
  let selectionWorker:Worker|null = null, ringSession = false;
  function clearSourceZone() { selectionReplies.clear(); sourceZone = null; zoneInspection = null; hasMore = true; ringSession = false; }
  function resetSourceInspection(newGeometry:boolean) {
    if (newGeometry) { selectionReplies.newSource(); sourceRevision = selectionReplies.revision; sourceZone = null; }
    else { selectionReplies.clear(); if (sourceZone) sourceZone = { ...sourceZone, ring:0 }; }
    ringSession = false; zoneInspection = null; topologyState = bytes && native() ? 'pending' : 'unavailable'; hasMore = true;
  }
  function toggleSelection() {
    selectionMode = !selectionMode; selectionReplies.clear(); ringSession = false;
    if (sourceZone) sourceZone = { ...sourceZone, ring:0 };
  }
  function inspectSourceZone() {
    if (!sourceZone || topologyState !== 'ready' || !selectionWorker) return;
    selectionWorker.postMessage({ type:ringSession ? 'zone' : 'inspect', token:selectionReplies.next(current), bodyId:sourceZone.previewBodyId, seed:sourceZone.seed, ring:sourceZone.ring, triangles:Array.from(sourceZone.triangles) });
  }
  function pickSourceTriangle(bodyId:string, triangle:number) {
    if (!selectionMode) return;
    mappedSource = false; selectionReplies.clear(); ringSession = false; sourceZone = { previewBodyId:bodyId, seed:triangle, triangles:[triangle], ring:0 }; zoneInspection = null; hasMore = true;
    inspectSourceZone();
  }
  function resizeSourceZone(delta:number) {
    if (!sourceZone || topologyState !== 'ready' || !selectionWorker) return;
    ringSession = true; sourceZone = { ...sourceZone, ring:Math.max(0,sourceZone.ring+delta) };
    inspectSourceZone();
  }
  function interpretSourceZone() {
    if (!selectionWorker || topologyState !== 'ready') return;
    selectionWorker.postMessage({ type:'interpret', token:selectionReplies.next(current), items:attributions(result?.bodies ?? []) });
  }
  async function prepareSelection(id:number, key:number, opts:Options, doc:DocumentPreview, revision:number) {
    try {
      const topology = await native()!.invoke<SelectionTopology>('selection_topology', { requestId:id, sourceId:key, options:opts });
      if (id !== current || sourceId !== key || revision !== sourceRevision || topology.request_id !== id || topology.source_id !== key) return;
      if (!selectionWorker) { topologyState = 'unavailable'; return; }
      selectionWorker.postMessage({ type:'prepare', token:selectionReplies.next(id), topology, bodyCounts:Object.fromEntries(doc.bodies.map(body => [body.id,body.mesh.triangles.length])) });
    } catch { if (id === current && sourceId === key && revision === sourceRevision) { topologyState = 'unavailable'; zoneInspection = null; } }
  }
  let stepDisplay = $state.raw<StepDisplay|null>(null);
  let stepView = $state(false), stepDocument = $state.raw<ResultFaces|null>(null), stepSelection = $state.raw<StepSelection|null>(null);
  let stepPending = $state(false), stepUnavailable = $state(false), mappedSource = $state(false);
  let stepSelectionPending = $state(false);
  let regionError = $state<string|null>(null);
  let sectionActive = $state(false);
  let sectionTarget=$state<'step'|'source'>('step');
  let sectionInput = $state<SectionInput>({basis:'xy',face_id:null,offset_mm:0,solid_ids:[]});
  let sectionMode=$state<SectionManipulationMode>('translate'),sectionDragging=$state(false),sectionDisplayFrame=$state<ProfileFrame|null>(null);
  let sectionFaceBase:{id:number;frame:ProfileFrame}|null=null,sectionDragStart:{input:SectionInput;frame:ProfileFrame|null}|null=null;
  let guidanceHints = $state<GuidedReport[]>([]);
  let guidedAttempt = $state<{ requestId:number; status:'pending'|'success'|'refused' }|null>(null);
  const guidedRefusal = $derived(isGuidedRefusal(guidedAttempt,current,!!error));
  const errorTitle = $derived(guidedRefusal ? 'guidance.notApplied' : 'workspace.failed');
  const eligibleGuidedFaces = $derived(!busy && !processing && result?.request_id === current && result.settings.reconstruction !== 'height_patch' ? guidedFaces(stepDocument,stepSelection,current,sourceId,guidanceHints.length) : null);
  const stepContextReveal = new ContextReveal();
  let stepWorker:Worker|null = null, pendingStepDocument:ResultFaces|null = null;
  const stepReplies = new SelectionReplies();
  const regionRequests = new RegionSelectionQueue();
  const paintQueue=new FacePaintQueue();
  let paintAck:{job:number;resolve:()=>void}|null=null;
  function cancelPaintQueue(){paintQueue.clear();}
  function paintStepFaces(ids:number[]) {
    const document=stepDocument,key=sourceId;
    if(!ids.length||!stepWorker||!document||document.request_id!==current||!stepView||busy||processing||sectionActive)return;
    stepReplies.clear();regionError=null;stepSelectionPending=true;faceProfiles.clear();
    paintQueue.add(ids,async batch=>{
      if(!stepWorker||stepDocument!==document||sourceId!==key||document.request_id!==current||!stepView)return;
      const token=stepReplies.next(current);
      await new Promise<void>(resolve=>{paintAck={job:token.job,resolve};try{stepWorker!.postMessage({type:'add',ids:batch,token});}catch{paintAck=null;paintQueue.clear();stepSelectionPending=false;regionError=t('stepInspection.regionUnavailable');resolve();}});
    });
  }
  function resetStepInspection() {
    cancelPaintQueue();
    regionError = null; stepContextReveal.next(null);
    stepReplies.newSource(); stepView = false; stepDocument = null; stepDisplay = null; stepSelection = null;
    pendingStepDocument = null; stepPending = false; stepUnavailable = false; mappedSource = false; stepSelectionPending = false;
    faceProfiles.clear();
    closeSection();
  }
  function stepCommand(type:'pick'|'grow'|'shrink'|'clear', id?:number, additive = false) {
    cancelPaintQueue();
    if (!stepWorker || !stepDocument || stepDocument.request_id !== current) return;
    regionError = null; stepSelectionPending = true; faceProfiles.clear();
    if(sectionActive&&sectionInput.basis==='face')sections.clear();
    stepWorker.postMessage({ type, token:stepReplies.next(current), id, additive });
  }
  async function selectStepRegion(faceId:number) {
    cancelPaintQueue();
    const bridge=native(), document=stepDocument, key=sourceId;
    if(!bridge||!document||document.request_id!==current||document.source_id!==key||!stepWorker||busy||processing||sectionActive)return;
    const token=stepReplies.next(current);
    const isCurrent=()=>stepReplies.accepts(token,current)&&sourceId===key&&stepDocument===document&&stepView;
    regionError=null;stepSelectionPending=true;faceProfiles.clear();
    try {
      const ids=await regionRequests.request(bridge,document,faceId,isCurrent);
      if(ids&&isCurrent()){if(ids.length===1)regionError=t('stepInspection.regionUnchanged');stepWorker?.postMessage({type:'select',ids,token});}
    } catch {
      if(isCurrent()){regionError=t('stepInspection.regionUnavailable');stepSelectionPending=false;}
    }
  }
  function clearStepSelection() { mappedSource = false; stepReplies.clear(); stepSelection = null; stepContextReveal.next(null); stepCommand('clear'); }
  function clearVisibleSelection() { if (stepView || mappedSource) clearStepSelection(); else clearSourceZone(); }
  function showSource() {
    cancelPaintQueue();
    stepReplies.clear();stepSelectionPending=false;regionError=null;
    if (stepPending) { stepReplies.clear(); pendingStepDocument = null; stepPending = false; }
    mappedSource = false; stepView = false; stepUnavailable = false;
    faceProfiles.clear();
    closeSection();
  }
  function showMappedSource() {
    cancelPaintQueue();
    if (!stepSelection?.faces.length || stepSelection.source.status !== 'exact' || !stepDocument || stepDocument.request_id !== current || stepDocument.source_id !== sourceId || !preview || preview.request_id !== current) return;
    stepReplies.clear();stepSelectionPending=false;regionError=null;
    mappedSource = true; stepView = false; stepUnavailable = false;
    faceProfiles.clear();
    closeSection();
  }
  async function showStepResult() {
    if (!result || result.request_id !== current || sourceId === null || !native() || stepPending) return;
    if(sectionActive&&sectionTarget==='source')closeSection();
    stepUnavailable = false; mappedSource = false;
    if (stepDocument) { stepView = true; selectionMode = true; return; }
    const id = current, key = sourceId, token = stepReplies.next(current);
    stepPending = true;
    try {
      const document = await native()!.invoke<ResultFaces>('result_faces', { requestId:id, sourceId:key });
      if (!stepReplies.accepts(token,current) || sourceId !== key) return;
      if (document.request_id !== id || document.source_id !== key) throw new Error('step.stale_preview');
      if (!stepWorker) throw new Error('step.worker_unavailable');
      pendingStepDocument = document;
      stepWorker.postMessage({ type:'prepare', token, document, bodyCounts:Object.fromEntries((preview?.bodies ?? []).map(body => [body.id,body.mesh.triangles.length])) });
    } catch {
      if (stepReplies.accepts(token,current)) { stepPending = false; stepUnavailable = true; stepView = false; }
    }
  }
  async function tryGuidedFamily(family:GuidedFamily) {
    if (stepSelectionPending) return;
    const faceIds = eligibleGuidedFaces;
    if (!faceIds) return;
    const sourceSelection = stepSelection?.source;
    if (sourceSelection?.status !== 'exact' || sourceSelection.bodies.length !== 1) return;
    const selectedSource = sourceSelection.bodies[0];
    sourceZone = { previewBodyId:selectedSource.previewBodyId, seed:selectedSource.triangles[0], triangles:[...selectedSource.triangles], ring:0 };
    await mutateGuidance('guided_convert', { faceIds, family });
  }
  function removeGuidance(hintId:number) { void mutateGuidance('remove_guidance', { hintId }); }
  function recomputeGuidance() { void mutateGuidance('recompute_guidance'); }
  function viewKey(e:KeyboardEvent) {
    if (e.key !== 'ArrowLeft' && e.key !== 'ArrowRight') return;
    e.preventDefault();
    const buttons = [...(e.currentTarget as HTMLElement).parentElement!.querySelectorAll<HTMLButtonElement>('button')];
    const target = e.key === 'ArrowLeft' ? buttons[0] : buttons[1];
    if (!target.disabled) { target.focus(); target.click(); }
  }
  type Internals = SourceBridge;
  let conversionProgress = $state<ConversionProgress|null>(null);
  $effect(() => {
    const id = current, source = sourceId, active = processing;
    conversionProgress = null;
    const bridge = native();
    if (bridge && source !== null && active?.requestId === id && active.phase === 'converting') {
      return watchConversionProgress(bridge, id, source, value => { conversionProgress = value; });
    }
  });
  const native = () => (window as unknown as { __TAURI_INTERNALS__?: Internals }).__TAURI_INTERNALS__;
  let largeTriangles = $state<number | null>(null);
  let largeAccepted = $state(false);
  let cancelling = $state(false);
  const canPrepareGuidance = $derived(!largeTriangles && canPrepareCorrection(result,current,options.reconstruction,!!native(),busy || !!processing) && (options.reconstruction === 'height_patch' || stepDocument?.source_correspondence.status !== 'exact'));
  let faceProfile = $state<FaceProfileState>({ response:null, geometry:null, pending:false, exporting:false, error:null, note:null });
  let profileGeometryWorker:ReturnType<typeof createFaceProfileGeometryWorker>|null = null;
  function faceProfileContext():FaceProfileContext|null {
    if (sectionActive || !stepView || stepSelectionPending || busy || processing || !result || result.request_id !== current || sourceId === null || !stepDocument || stepDocument.request_id !== current || stepDocument.source_id !== sourceId || stepSelection?.faces.length !== 1) return null;
    const faceId = stepSelection.faces[0];
    return stepDocument.faces.find(face => face.id === faceId)?.family === 'plane' ? { requestId:current, sourceId, faceId } : null;
  }
  function prepareProfileGeometry(profile:PlanarFaceProfile) {
    return profileGeometryWorker?.prepare(profile) ?? Promise.reject({code:'engine.task',message:'Planar face display worker is unavailable.'});
  }
  const faceProfiles = createFaceProfiles(faceProfile,{ native, context:faceProfileContext, name:()=>name, prepare:prepareProfileGeometry, clearPrepared:()=>profileGeometryWorker?.clear(), initialSelectionId:Date.now()*1000 });
  const profileEligible = $derived(!!faceProfileContext());
  $effect(() => { void faceProfiles.sync(); });
  let sectionState=$state<SectionState>({response:null,frame:null,geometry:null,pending:false,exporting:false,error:null,note:null});
  let sourcePreparationState=$state<SourceSectionPreparationState>({response:null,pending:false,error:null});
  let sectionGeometryWorker:ReturnType<typeof createFaceProfileGeometryWorker>|null=null;
  const sectionSolids=$derived(sectionTarget==='source'?sourcePreparationState.response?.metadata?.bodies.map(body=>body.solid_id)??[]:stepDocument?[...new Set(stepDocument.faces.map(face=>face.solid_id))].sort((a,b)=>a-b):[]);
  const sectionBounds=$derived(sectionTarget==='source'?sourcePreparationState.response?.metadata?.bounds_mm??null:result?.cad.bounds_mm??null);
  function sourcePreparationContext():SourceSectionPreparationContext|null{
    return sectionActive&&sectionTarget==='source'&&!stepView&&!mappedSource&&!stepPending&&!busy&&!processing&&bytes&&sourceId!==null&&uploadedSourceId===sourceId&&preview?.request_id===current?{requestId:current,sourceId,options:serializableOptions(options)}:null;
  }
  const sourcePreparation=createSourceSectionPreparation(sourcePreparationState,{native,context:sourcePreparationContext,initialPreparationId:Date.now()*1000,onReady:response=>{sectionInput=initialSourceSectionInput(response.metadata,response.preparation_id);sectionDisplayFrame=localSectionFrame(sectionInput,null);}});
  $effect(()=>{void sourcePreparation.sync();});
  function selectedPlaneFace():number|null {
    if(stepSelectionPending||!stepDocument||stepDocument.request_id!==current||stepSelection?.faces.length!==1)return null;
    const id=stepSelection.faces[0];return stepDocument.faces[id]?.family==='plane'?id:null;
  }
  function sectionContext():SectionContext|null {
    if(sectionDragging||!sectionActive||busy||processing||sourceId===null||!sectionInput.solid_ids.length||!Number.isFinite(sectionInput.offset_mm)||Math.abs(sectionInput.offset_mm)>1e9)return null;
    if(sectionTarget==='source'){
      const prepared=sourcePreparationState.response;
      if(stepView||mappedSource||!prepared?.metadata||sourcePreparationState.pending||prepared.request_id!==current||prepared.source_id!==sourceId||sectionInput.target!=='source'||sectionInput.preparation_id!==prepared.preparation_id||sectionInput.basis==='face')return null;
    }else if(!stepView||!result||result.request_id!==current||!stepDocument||stepDocument.request_id!==current||stepDocument.source_id!==sourceId||(sectionInput.basis==='face'&&(stepSelectionPending||sectionInput.face_id===null)))return null;
    return {requestId:current,sourceId,input:sectionInput};
  }
  const sections=createSections(sectionState,{native,context:sectionContext,name:()=>name,prepare:(profile:SectionProfile)=>sectionGeometryWorker?.prepareSection(profile)??Promise.reject({code:'engine.task',message:'Section display worker unavailable.'}),clearPrepared:()=>sectionGeometryWorker?.clear(),schedule:(callback,delay)=>setTimeout(callback,delay),clearSchedule:handle=>clearTimeout(handle as ReturnType<typeof setTimeout>),initialSectionId:Date.now()*1000,onFrame:(frame,input)=>{if(input.basis==='face'&&input.face_id!==null)sectionFaceBase={id:input.face_id,frame:offsetSectionFrame(frame,-input.offset_mm)};if(!sectionDragging)sectionDisplayFrame=cloneSectionFrame(frame);}});
  $effect(()=>{void sections.sync();});
  function closeSection(){sectionActive=false;sectionDragging=false;sectionDragStart=null;sectionDisplayFrame=null;sectionFaceBase=null;sections.clear();sourcePreparation.clear();}
  function canOpenSection(){return !!native()&&!busy&&!processing&&!stepPending&&(stepView?!!stepDocument&&stepDocument.request_id===current&&!!result&&result.request_id===current:!mappedSource&&!!bytes&&sourceId!==null&&uploadedSourceId===sourceId&&preview?.request_id===current);}
  function toggleSection(){
    if(sectionActive){closeSection();return;}
    if(!canOpenSection())return;
    sectionTarget=stepView?'step':'source';
    sectionInput=sectionTarget==='step'?{basis:'xy',face_id:null,offset_mm:(result!.cad.bounds_mm[0][2]+result!.cad.bounds_mm[1][2])/2,solid_ids:[...new Set(stepDocument!.faces.map(face=>face.solid_id))].sort((a,b)=>a-b)}:{target:'source',basis:'xy',face_id:null,offset_mm:0,solid_ids:[]};
    sectionDisplayFrame=sectionTarget==='step'?localSectionFrame(sectionInput,null):null;sectionMode='translate';
    faceProfiles.clear();sectionActive=true;
  }
  function changeSection(input:SectionInput){sectionInput=cloneSectionInput(input);sectionDisplayFrame=localSectionFrame(input,sectionFaceBase?.id===input.face_id?sectionFaceBase.frame:null);sections.clear();}
  function followSectionFace(){if(sectionActive&&sectionTarget==='step'&&sectionInput.basis==='face')changeSection({...sectionInput,face_id:selectedPlaneFace()});}
  function transformSection(frame:ProfileFrame){if(sectionActive&&sectionDragging){sectionInput=transformedSectionInput(sectionInput,frame);sectionDisplayFrame=cloneSectionFrame(frame);}}
  function sectionDrag(active:boolean,cancelled=false){
    if(active){sectionDragStart={input:cloneSectionInput(sectionInput),frame:sectionDisplayFrame?cloneSectionFrame(sectionDisplayFrame):null};sectionDragging=true;sections.clear();}
    else{if(cancelled&&sectionDragStart&&sectionActive){sectionInput=sectionDragStart.input;sectionDisplayFrame=sectionDragStart.frame;}sectionDragStart=null;sectionDragging=false;}
  }
  function resetSectionPlane(){if(sectionBounds)changeSection({...sectionInput,basis:'xy',face_id:null,custom_frame:null,offset_mm:(sectionBounds[0][2]+sectionBounds[1][2])/2});}
  function sectionBodyName(id:number){const body=sourcePreparationState.response?.metadata?.bodies.find(body=>body.solid_id===id);return body?t('sourceSection.body',{n:id+1,name:bodyLabel({id:body.analysis_body_id,name:body.name,provenance:body.provenance},t('workspace.stlBody'),(object,instance)=>t('workspace.objectBody',{object,instance}))}):t('section.solid',{n:id+1});}
  const engineKeys: Record<string, Key> = { 'recognition.height_patch_unproven':'engine.height_patch_unproven', 'worker.busy':'engine.workerBusy', 'worker.memory_limit':'engine.workerMemory', 'worker.monitor_failed':'engine.workerMonitor', 'worker.invalid_input':'engine.workerProtocol', 'worker.input_changed':'engine.workerProtocol', 'worker.invalid_job':'engine.workerProtocol', 'worker.request_invalid':'engine.workerProtocol', 'worker.request_too_large':'engine.workerProtocol', 'worker.job_not_fresh':'engine.workerProtocol', 'worker.serialization':'engine.workerProtocol', 'worker.error':'engine.workerProtocol', 'worker.source_too_large':'engine.input_too_large', "worker.unavailable":"engine.workerUnavailable", "worker.confirmation_required":"engine.workerConfirmation", "worker.input_invalid":"engine.workerInput", "worker.protocol":"engine.workerProtocol", "worker.output_too_large":"engine.workerOutput", "worker.failed":"engine.workerFailed", "worker.cancelled":"engine.workerCancelled", "worker.timeout":"engine.workerTimeout", "worker.io":"engine.workerIo", "worker.allocation":"engine.workerAllocation", 'guidance.family_unproven':'guidance.family_unproven', 'guidance.region_unavailable':'guidance.region_unavailable', 'search.analytic_mode_required':'parameterSearch.analyticRequired', 'search.guidance_active':'parameterSearch.guidanceActive', 'guidance.limit':'guidance.limit', 'guidance.overlapping_indications':'guidance.overlapping_indications', 'guidance.unknown_indication':'guidance.unknown_indication', 'guidance.provenance_unavailable':'guidance.provenance_unavailable', 'guidance.invalid_selection':'guidance.invalid_selection', 'guidance.multiple_bodies':'guidance.multiple_bodies', 'guidance.plane_incomplete':'guidance.plane_incomplete', 'guidance.plane_unproven':'guidance.plane_unproven', 'guidance.plane_unsupported_boundary':'guidance.plane_unsupported_boundary', 'guidance.cone_unproven':'guidance.cone_unproven', 'guidance.ambiguous_cone':'guidance.ambiguous_cone', 'guidance.unsupported_family':'guidance.unsupported_family', 'guidance.cylinder_unproven':'guidance.cylinder_unproven', 'guidance.ambiguous_cylinder':'guidance.ambiguous_cylinder', 'threemf.invalid_archive':'engine.m3_invalid_archive', 'threemf.archive_limit':'engine.m3_archive_limit', 'threemf.unsupported_compression':'engine.m3_unsupported_compression', 'threemf.invalid_path':'engine.m3_invalid_path', 'threemf.duplicate_part':'engine.m3_duplicate_part', 'threemf.missing_part':'engine.m3_missing_part', 'threemf.invalid_xml':'engine.m3_invalid_xml', 'threemf.xml_limit':'engine.m3_xml_limit', 'threemf.invalid_package':'engine.m3_invalid_package', 'threemf.external_resource':'engine.m3_external_resource', 'threemf.invalid_transform':'engine.m3_invalid_transform', 'threemf.non_finite':'engine.m3_non_finite', 'threemf.complexity_limit':'engine.m3_complexity_limit', 'threemf.invalid_reference':'engine.m3_invalid_reference', 'threemf.invalid_index':'engine.m3_invalid_index', 'threemf.empty_model':'engine.m3_empty_model', 'threemf.unsupported_version':'engine.m3_unsupported_version', 'threemf.unsupported_extension':'engine.m3_unsupported_extension', 'threemf.invalid_unit':'engine.m3_invalid_unit', 'threemf.duplicate_id':'engine.m3_duplicate_id', 'threemf.unsupported_object':'engine.m3_unsupported_object', 'mesh.invalid_index':'engine.m3_mesh_invalid_index', 'input.too_many_bodies':'engine.m3_too_many_bodies', 'cad.bodies_not_separated':'workspace.notSeparated', 'engine.source_metadata':'engine.source_metadata', 'engine.source_transport':'engine.source_transport', 'engine.source_busy':'engine.source_busy', 'engine.source_conflict':'engine.source_conflict', 'engine.source_allocation':'engine.source_allocation', 'engine.stale':'engine.engine_stale', 'engine.state':'engine.engine_state', 'io.extension':'engine.io_extension', 'io.file':'engine.io_file', 'io.path':'engine.io_path', 'engine.task':'engine.engine_task', 'validation.analytic_mismatch':'engine.validation_analytic_mismatch', 'recognition.conic_unproven':'engine.recognition_conic_unproven', 'recognition.partial_unproven':'engine.recognition_partial_unproven', 'validation.distance_budget':'engine.distance_budget', 'validation.distance_resolution':'engine.distance_resolution', 'recognition.disabled':'engine.recognition_disabled', 'recognition.unsupported':'engine.recognition_unsupported', 'recognition.too_complex':'engine.recognition_too_complex', 'recognition.invalid_analysis':'engine.recognition_invalid_analysis', "options.invalid_tolerance": "engine.options_invalid_tolerance", "mesh.empty": "engine.mesh_empty", "mesh.non_finite": "engine.mesh_non_finite", "mesh.invalid_volume": "engine.mesh_invalid_volume", "mesh.coordinate_range": "engine.mesh_coordinate_range", "mesh.degenerate_triangle": "engine.mesh_degenerate_triangle", "mesh.duplicate_triangle": "engine.mesh_duplicate_triangle", "mesh.non_manifold_edge": "engine.mesh_non_manifold_edge", "mesh.open": "engine.mesh_open", "mesh.inconsistent_orientation": "engine.mesh_inconsistent_orientation", "mesh.multiple_components": "engine.mesh_multiple_components", "mesh.non_manifold_vertex": "engine.mesh_non_manifold_vertex", "input.too_large": "engine.input_too_large", "input.too_many_triangles": "engine.input_too_many_triangles", "stl.invalid_length": "engine.stl_invalid_length", "stl.invalid_ascii": "engine.stl_invalid_ascii", "planar.too_complex": "engine.planar_too_complex", "planar.invalid_contour": "engine.planar_invalid_contour", "validation.bounds_mismatch": "engine.validation_bounds_mismatch", "validation.volume_mismatch": "engine.validation_volume_mismatch" };
  const diagnosticKeys: Record<string, Key> = { 'recognition.height_patch_fitted':'engine.height_patch_fitted', 'recognition.bored_prism_projection':'engine.bored_prism_projection', 'recognition.unequal_pocket_fitted':'engine.unequal_pocket_fitted', 'recognition.unequal_pocket_adjusted':'engine.oblique_pocket_adjusted', 'recognition.adjacent_pocket_fitted':'engine.adjacent_pocket_fitted', 'recognition.trimmed_tori_fitted':'engine.trimmed_tori_fitted', 'recognition.u_pocket_fitted':'engine.u_pocket_fitted', 'recognition.closed_pocket_fitted':'engine.closed_pocket_fitted', 'recognition.adjacent_pocket_adjusted':'engine.oblique_pocket_adjusted', 'recognition.double_oblique_pocket_fitted':'engine.double_oblique_pocket_fitted', 'recognition.oblique_pocket_fitted':'engine.oblique_pocket_fitted', 'recognition.oblique_pocket_adjusted':'engine.oblique_pocket_adjusted', 'recognition.toric_sector_endpoints_projected':'engine.toric_sector_endpoints_projected', 'recognition.toric_sector_fitted':'engine.toric_sector_fitted', 'reconstruction.fitted_revolution':'engine.fitted_revolution', 'recognition.linear_blend_endpoints_projected':'engine.linear_blend_endpoints_projected', 'recognition.partial_conical_chain':'engine.partial_conical_chain', 'auto.faceted_base':'engine.auto_faceted_base', 'auto.planar_regions_split':'engine.auto_planar_regions_split', 'auto.regions_faceted':'engine.auto_region_faceted', 'guidance.auto_applied':'guidance.auto_applied', 'guidance.sphere_applied':'guidance.sphere_applied', 'guidance.torus_applied':'guidance.torus_applied', 'guidance.plane_verified':'guidance.plane_verified', 'guidance.plane_applied':'guidance.plane_applied', 'guidance.plane_residual':'guidance.plane_residual',  'guidance.cone_applied':'guidance.cone_applied', 'guidance.complete_cone_verified':'guidance.cone_applied', 'guidance.cylinder_applied':'guidance.cylinder_applied', 'guidance.complete_cylinder_verified':'guidance.cylinder_applied', 'recognition.rounded_box_intent_assumed':'engine.rounded_box_intent_assumed', 'recognition.rounded_box_coverage':'engine.rounded_box_coverage', 'recognition.rounded_intent_assumed':'engine.rounded_intent_assumed', 'recognition.conic_intent_assumed':'engine.conic_intent_assumed', 'precision.custom_tolerances':'engine.custom_tolerances', 'document.product_names_adapted':'engine.product_names_adapted', 'threemf.metadata_ignored':'engine.m3_metadata_ignored', 'document.separate_bodies':'workspace.bodiesSeparated', 'recognition.circular_intent_assumed':'engine.recognition_circular_intent', 'recognition.bound_scope':'engine.recognition_bound_scope', "validation.native_required": "engine.diag_validation_native_required", "mesh.orientation_reversed": "engine.diag_mesh_orientation_reversed", "planar.approximate": "engine.diag_planar_approximate" };
  function errorText(e: unknown): string {
    if (typeof e !== 'object' || !e || !('message' in e)) return `${t('engine.unknownDetail')}: ${String(e)}`;
    const raw = e as { code?: string; message: string; body?: { id: string; name: string; detail: string } | null };
    const context = errorContext(raw, t('workspace.stlBody'), id => {
      const body = preview?.bodies.find(body => body.id === id || id.startsWith(`${body.id}/component/`));
      return body ? bodyText(body) : undefined;
    });
    const err = { ...raw, message: context.detail };
    const withBody = (text: string) => context.prefix + text;
    const triangleLimit = triangleLimitCounts(err.code, err.message);
    if (triangleLimit) {
      const format = new Intl.NumberFormat(i18n.locale);
      return withBody(`${t('engine.input_triangle_count', {count:format.format(triangleLimit.count), limit:format.format(triangleLimit.limit)})} (${err.code})`);
    }
    if (err.code === 'recognition.surface_unproven') return withBody(`${t('engine.recognition_surface_unproven')} (${err.code}) — ${t('engine.unknownDetail')}: ${err.message}`);
    if (err.code === 'recognition.conic_unproven') return withBody(`${t('engine.recognition_conic_unproven')} (${err.code}) — ${t('engine.unknownDetail')}: ${err.message}`);
    if (err.code === 'cad.invalid_model' && err.message === "Le contour d'une face dépasse la tolérance de planéité du noyau.") return withBody(`${t('engine.cad_contour_planarity')} (${err.code})`);
    if (err.code === 'recognition.deviation_exceeded') {
      const values = [...err.message.matchAll(/([+-]?\d+(?:\.\d+)?(?:e[+-]?\d+)?)\s*mm/gi)].map(m => m[1]);
      if (values.length >= 2) return withBody(`${t('engine.recognition_deviation', { bound:values[0], budget:values[1] })} (${err.code})`);
      if (options.reconstruction === 'height_patch') return withBody(`${t('engine.height_patch_budget')} (${err.code})`);
    }
    if (err.code === 'validation.distance_exceeded') {
      const value = err.message.match(/([+-]?\d+(?:\.\d+)?(?:e[+-]?\d+)?)\s*mm/i)?.[1];
      if (value) return withBody(`${t('engine.distance_exceeded', { limit:value })} (${err.code})`);
    }
    const detailed: Record<string, Key> = { 'validation.distance_invalid_mesh':'engine.distance_invalid_mesh', 'cad.tessellation_failed':'engine.tessellation_failed', 'cad.tessellation_too_complex':'engine.tessellation_too_complex' };
    if (err.code && detailed[err.code]) return withBody(`${t(detailed[err.code])} (${err.code}) — ${t('engine.unknownDetail')}: ${err.message}`);
    if (err.code === 'cad.bodies_not_separated') return withBody(`${t('workspace.notSeparated')} — ${t('engine.unknownDetail')}: ${err.message} (${err.code})`);
    const key = err.code && engineKeys[err.code];
    if (key && err.code && (err.code.startsWith('threemf.') || err.code === 'mesh.invalid_index' || err.code === 'input.too_many_bodies')) return withBody(`${t(key)} (${err.code}) — ${t('engine.unknownDetail')}: ${err.message}`);
    return withBody((key ? t(key) : `${t('engine.unknownDetail')}: ${err.message}`) + (err.code ? ` (${err.code})` : ''));
  }
  const parameterReplies = new ParameterSearchReplies();
  let parameterProposal = $state<{token:SearchToken; response:ParameterSearchResponse}|null>(null);
  let parameterError = $state<unknown>(null);
  const allowParameterSearch = $derived(options.reconstruction !== 'planar' && options.reconstruction !== 'height_patch');
  const canSearch = $derived(!!bytes && !!native() && sourceId !== null && uploadedSourceId === sourceId && preview?.request_id === current && !busy && !processing && allowParameterSearch && !guidanceHints.length && !guidedAttempt && validPreferences(options));
  function clearParameterSearch() { parameterReplies.clear(); parameterProposal = null; parameterError = null; }
  async function searchParameters() {
    if (!canSearch || sourceId === null) return;
    const id = current, key = sourceId, opts = serializableOptions(options), token = parameterReplies.begin(id,key,opts);
    parameterProposal = null; parameterError = null; busy = true;
    processing = {requestId:id, phase:'searching', startedAt:performance.now()};
    try {
      const response = await native()!.invoke<ParameterSearchResponse>('search_parameters', {requestId:id,sourceId:key,options:opts});
      if (parameterReplies.accepts(token,current,sourceId,options,response.request_id)) parameterProposal = {token,response};
    } catch (e) {
      if (parameterReplies.accepts(token,current,sourceId,options)) parameterError = e;
    } finally {
      if (processing?.requestId === id && processing.phase === 'searching') processing = null;
      busy = false;
      if (queued) void run();
    }
  }
  function applyParameterProposal() {
    const proposal = parameterProposal;
    if (!canSearch || !proposal || !parameterReplies.accepts(proposal.token,current,sourceId,options,proposal.response.request_id)) return;
    const value = proposedTolerance(proposal.response.report);
    if (value !== null && value !== options.cylinder_fit_tolerance_mm) change({cylinder_fit_tolerance_mm:value});
  }
  // Façade vivante : le contrôleur et les workers partagent les mêmes runes et le verrou moteur.
  const workflowState: WorkflowState = {
    get largeTriangles() { return largeTriangles; },
    set largeTriangles(value) { largeTriangles = value; },
    get largeAccepted() { return largeAccepted; },
    set largeAccepted(value) { largeAccepted = value; },
    get cancelling() { return cancelling; },
    set cancelling(value) { cancelling = value; },
    get options() { return options; },
    set options(value) { options = value; },
    get bytes() { return bytes; },
    set bytes(value) { bytes = value; },
    get name() { return name; },
    set name(value) { name = value; },
    get format() { return format; },
    set format(value) { format = value; },
    get current() { return current; },
    set current(value) { current = value; },
    get fileToken() { return fileToken; },
    set fileToken(value) { fileToken = value; },
    get sourceId() { return sourceId; },
    set sourceId(value) { sourceId = value; },
    get uploadedSourceId() { return uploadedSourceId; },
    set uploadedSourceId(value) { uploadedSourceId = value; },
    get sourceRevision() { return sourceRevision; },
    set sourceRevision(value) { sourceRevision = value; },
    get result() { return result; },
    set result(value) { result = value; },
    get preview() { return preview; },
    set preview(value) { preview = value; },
    get selectedId() { return selectedId; },
    set selectedId(value) { selectedId = value; },
    get errorState() { return errorState; },
    set errorState(value) { errorState = value; },
    get noteKey() { return noteKey; },
    set noteKey(value) { noteKey = value; },
    get recognitionFailed() { return recognitionFailed; },
    set recognitionFailed(value) { recognitionFailed = value; },
    get busy() { return busy; },
    set busy(value) { busy = value; },
    get queued() { return queued; },
    set queued(value) { queued = value; },
    get processing() { return processing; },
    set processing(value) { processing = value; },
    get topologyState() { return topologyState; },
    set topologyState(value) { topologyState = value; },
    get guidanceHints() { return guidanceHints; },
    set guidanceHints(value) { guidanceHints = value; },
    get guidedAttempt() { return guidedAttempt; },
    set guidedAttempt(value) { guidedAttempt = value; },
    get fitSignal() { return fitSignal; },
    set fitSignal(value) { fitSignal = value; },
  };
  const workflow = createWorkflow(workflowState, {
    native,
    serializableOptions,
    clearParameterSearch,
    resetSourceInspection,
    resetStepInspection,
    prepareSelection,
    interpretSourceZone,
    // L'aperçu garde ses propres contrôles de fraîcheur et ne bloque pas l'export validé.
    onResultReady: () => { void showStepResult(); },
    now: () => performance.now(),
    schedule: (callback, delay) => setTimeout(callback, delay),
    clearSchedule: handle => clearTimeout(handle as ReturnType<typeof setTimeout>),
  });
  const { invalidate, run, load, mutateGuidance, exportStep, reconstruct } = workflow;
  const canReconstruct = $derived(workflow.canReconstruct());
  function commitOptions() {
    const previous = commitSnapshot(options, committed); if (!previous) return;
    history.push(previous); if (history.length > 50) history.shift();
    future = []; committed = { ...options }; saveOptions();
  }
  function change(patch: Partial<Options>) {
    const unitChanged = patch.input_unit !== undefined && patch.input_unit !== options.input_unit;
    options = { ...options, ...patch }; commitOptions(); invalidate(!unitChanged);
  }
  type Tolerance = 'weld_tolerance_mm' | 'plane_tolerance_mm' | 'cylinder_fit_tolerance_mm' | 'max_deviation_mm';
  let scrubbedTolerance: Tolerance | null = null;
  function changeTolerance(field: Tolerance, value: number) {
    if (value === options[field]) return;
    if (field === scrubbedTolerance) {
      options = { ...options, [field]: value };
      invalidate(true);
    } else {
      change({ [field]: value });
    }
  }
  function finishToleranceScrub() {
    scrubbedTolerance = null;
    commitOptions();
  }
  function saveOptions() { if (!validPreferences(committed)) return; try { localStorage.setItem('fabstep.options.v2', JSON.stringify(committed)); } catch { /* optionnel */ } }
  function undo(redo = false) {
    if (!sameOptions(options, committed)) { const unitChanged = options.input_unit !== committed.input_unit; options = { ...committed }; invalidate(!unitChanged); return; }
    const from = redo ? future : history, to = redo ? history : future;
    const prev = from.pop(); if (prev) { const unitChanged = prev.input_unit !== options.input_unit; to.push({ ...committed }); committed = { ...prev }; options = { ...prev }; saveOptions(); invalidate(!unitChanged); }
  }
  const diagnosticText = (code: string, fallback: string, body: BodyResult | null = selection) => { if (code === 'recognition.rounded_coverage' && body?.rounded_recognition) return t('engine.rounded_coverage', { spheres:body.rounded_recognition.report.spheres, tori:body.rounded_recognition.report.tori, cylinders:body.rounded_recognition.report.cylinders, cones:body.rounded_recognition.report.cones, planes:body.rounded_recognition.model.revolved.mixed.planes.length }); if (code === 'recognition.revolved_coverage' && body?.revolved_recognition) return t('engine.revolved_coverage', { cones:body.revolved_recognition.report.cones, cylinders:body.revolved_recognition.report.cylinders, planes:body.revolved_recognition.model.mixed.planes.length }); if (code === 'recognition.mixed_coverage' && body?.mixed_recognition) return t('engine.mixed_coverage', { cylinders:body.mixed_recognition.report.cylinders, planes:body.mixed_recognition.model.planes.length }); if (code === 'mesh.vertices_moved') { const distance = fallback.match(/([+-]?\d+(?:\.\d+)?(?:e[+-]?\d+)?)\s*mm/i)?.[1]; if (distance) return t('engine.verticesMoved', { distance }); } return diagnosticKeys[code] ? t(diagnosticKeys[code]) : `${t('engine.unknownDetail')}: ${fallback}`; };
  const notices = $derived.by(() => {
    const documentNotices = (result?.diagnostics ?? preview?.diagnostics ?? []).map(diagnostic => ({ diagnostic, body:null as BodyResult|null, index:0 }));
    const bodies = result?.bodies ?? [];
    return [...documentNotices, ...bodies.flatMap(body => body.analysis.report.diagnostics.map(diagnostic => ({ diagnostic, body, index:result?.bodies.indexOf(body) ?? 0 })))];
  });
  // Les avertissements de tous les corps restent visibles même si un seul corps est inspecté.
  // Les comptes de couverture et remarques techniques restent consultables dans Détails.
  const fmt = (n: number) => new Intl.NumberFormat(i18n.locale === 'fr' ? 'fr-FR' : 'en-GB', { maximumFractionDigits:6 }).format(n);
  const selectedBody = $derived(stepView || mappedSource ? null : result?.bodies.find(body => body.id === selectedId) ?? null);
  const selectedPreview = $derived(stepView || mappedSource ? null : preview?.bodies.find(body => body.id === selectedId) ?? null);
  const selection = $derived(stepView || mappedSource ? null : selectedBody ?? (selectedId ? null : result?.bodies.length === 1 ? result.bodies[0] : null));
  const report = $derived(selection?.recognition?.report ?? selection?.mixed_recognition?.report ?? selection?.revolved_recognition?.report ?? selection?.rounded_recognition?.report ?? selection?.rounded_box_recognition?.report ?? null);
  const heightPatches = $derived((selection ? [selection] : result?.bodies ?? []).flatMap(body => body.height_patch_recognition ? [body.height_patch_recognition] : []));
  const heightBound = $derived(heightPatches.length ? Math.max(...heightPatches.map(r => r.report.max_deviation_bound_mm)) : null);
  const metrics = $derived(selection?.cad ?? result?.cad ?? null);
  const sourceTriangles = $derived(selection ? selection.analysis.report.input_triangles : result ? result.bodies.reduce((sum, body) => sum + body.analysis.report.input_triangles, 0) : selectedPreview ? selectedPreview.mesh.triangles.length : preview?.bodies.reduce((sum, body) => sum + body.mesh.triangles.length, 0) ?? null);
  const sourceVertices = $derived(selection ? selection.analysis.report.vertices : result ? result.bodies.reduce((sum, body) => sum + body.analysis.report.vertices, 0) : selectedPreview ? selectedPreview.mesh.vertices.length : preview?.bodies.reduce((sum, body) => sum + body.mesh.vertices.length, 0) ?? null);
  const unitNames: Record<string, Key> = { millimeter:'workspace.unitMillimeter', mm:'workspace.unitMillimeter', micron:'workspace.unitMicron', centimeter:'workspace.unitCentimeter', inch:'workspace.unitInch', foot:'workspace.unitFoot', meter:'workspace.unitMeter' };
  const unitText = (unit: string) => unitNames[unit] ? t(unitNames[unit]) : unit;
  const bodyText = (body: { id:string; name:string; provenance?: import('./lib/types').BodyProvenance }) => bodyLabel(body, t('workspace.stlBody'), (object, instance) => t('workspace.objectBody', { object, instance }));
  const precise = (n: number) => new Intl.NumberFormat(i18n.locale === 'fr' ? 'fr-FR' : 'en-GB', { maximumSignificantDigits: 6 }).format(n);
  const settingValue = (n: number) => i18n.locale === 'fr' ? String(n).replace('.', ',') : String(n);
  const measuredValue = (n: number) => settingValue(Number(n.toPrecision(6)));
  const dims = $derived(metrics ? metrics.bounds_mm[1].map((v,i) => fmt(v - metrics!.bounds_mm[0][i])).join(' × ') : '—');
  function key(e: KeyboardEvent) { if (showHelp) return; if (e.key === '?' && !['INPUT','SELECT','TEXTAREA'].includes((e.target as HTMLElement)?.tagName)) { e.preventDefault(); showHelp = true; return; } const editing = ['INPUT','SELECT','TEXTAREA'].includes((e.target as HTMLElement)?.tagName); if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'o') { e.preventDefault(); input.click(); } else if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'z' && !editing) { e.preventDefault(); undo(e.shiftKey); } else if (!editing && e.key === 'Escape') { clearVisibleSelection(); } else if (!editing && e.key.toLowerCase() === 'f') fitSignal++; }
  onMount(() => {
    try { profileGeometryWorker = createFaceProfileGeometryWorker(new Worker(new URL('./lib/face-profile.worker.ts',import.meta.url),{type:'module'})); }
    catch { /* Le refus du profil ne retire pas le STEP validé. */ }
    try {sectionGeometryWorker=createFaceProfileGeometryWorker(new Worker(new URL('./lib/face-profile.worker.ts',import.meta.url),{type:'module'}));}
    catch {/* L’export STEP reste disponible si l’aperçu de section échoue. */}
    try {
      selectionWorker = new Worker(new URL('./lib/selection.worker.ts',import.meta.url), { type:'module' });
      selectionWorker.onmessage = (event:MessageEvent<{ type:string; token:SelectionToken; inspection?:ZoneInspection; triangles?:number[]; hasMore?:boolean }>) => {
        const message = event.data;
        if (message.token.revision !== sourceRevision || message.token.requestId !== current) return;
        if (message.type === 'unavailable') { topologyState = 'unavailable'; zoneInspection = null; return; }
        if (message.type === 'ready') { topologyState = 'ready'; interpretSourceZone(); return; }
        if (message.type === 'interpreted') { inspectSourceZone(); return; }
        if (!selectionReplies.accepts(message.token,current)) return;
        if (message.type === 'error') { topologyState = 'unavailable'; zoneInspection = null; return; }
        if (!sourceZone) return;
        if (message.type === 'zone' && message.triangles) { sourceZone = { ...sourceZone, triangles:message.triangles }; hasMore = message.hasMore ?? false; }
        if (message.inspection) zoneInspection = message.inspection;
      };
      selectionWorker.onerror = () => { topologyState = 'unavailable'; zoneInspection = null; };
    } catch { topologyState = 'unavailable'; }
    try {
      stepWorker = new Worker(new URL('./lib/step-selection.worker.ts',import.meta.url), { type:'module' });
      stepWorker.onmessage = (event:MessageEvent<{ type:string; token:SelectionToken; selection?:StepSelection; display?:StepDisplay }>) => {
        const message = event.data;
        if(paintAck?.job===message.token.job){const ack=paintAck;paintAck=null;ack.resolve();}
        if (!stepReplies.accepts(message.token,current)) return;
        if (message.type === 'ready' && pendingStepDocument) { stepDocument = pendingStepDocument; stepDisplay = message.display ?? null; pendingStepDocument = null; stepPending = false; stepView = true; selectionMode = true; stepSelection = message.selection ?? null; stepSelectionPending = false; }
        else if (message.type === 'selection' && message.selection) { stepSelection = message.selection; if (!stepSelection.faces.length) stepContextReveal.next(null); stepSelectionPending = false; followSectionFace(); }
        else if (message.type === 'error') { stepPending = false; stepUnavailable = true; stepView = false; stepDocument = null; stepDisplay = null; pendingStepDocument = null; stepSelection = null; mappedSource = false; stepSelectionPending = false; faceProfiles.clear(); closeSection(); }
      };
      stepWorker.onerror = () => { cancelPaintQueue();paintAck?.resolve();paintAck=null;stepWorker?.terminate(); stepWorker = null; resetStepInspection(); stepUnavailable = !!result; };
    } catch { /* L'aperçu source et l'export restent disponibles. */ }
    window.addEventListener('keydown', key);
    return () => { cancelPaintQueue();paintAck?.resolve();paintAck=null;window.removeEventListener('keydown', key); selectionWorker?.terminate(); stepWorker?.terminate(); faceProfiles.dispose(); profileGeometryWorker?.dispose(); sections.dispose();sourcePreparation.dispose();sectionGeometryWorker?.dispose();workflow.dispose(); };
  });
</script>
<svelte:window
  ondragover={(e) => { e.preventDefault(); dragging = true; }}
  ondragleave={(e) => { if (!e.relatedTarget) dragging = false; }}
  ondrop={(e) => { e.preventDefault(); dragging = false; if (e.dataTransfer?.files[0]) void load(e.dataTransfer.files[0]); }}
/>
<div class="app">
  <header class="top">
    <div class="brand">
      <img src="/logo.svg" width="24" height="24" alt=""/>
      <strong>FabSTEP</strong>
      <span>{t('workspace.tagline')}</span>
    </div>
    <button class="btn" class:primary={!result} onclick={() => input.click()} title="⌘/Ctrl+O">
      <FileUp size={14}/>
      {t('workspace.open')}
    </button>
    <span class="filename" title={name || t('workspace.noFile')}>{name || t('workspace.noFile')}</span>
    <div class="spacer"></div>
    <div class="switch" role="group" aria-label={t('workspace.language')}>
      {#each LOCALES as { id, label }}
        <button lang={id} aria-label={label} aria-pressed={i18n.locale === id} class:active={i18n.locale === id} onclick={() => i18n.set(id)}>
          {id.toUpperCase()}
        </button>
      {/each}
    </div>
    <div class="switch" role="group" aria-label={t('workspace.theme')}>
      {#each ['system','light','dark'] as th}
        {@const Icon = th === 'system' ? Monitor : th === 'light' ? Sun : Moon}
        <button
          aria-pressed={theme.choice === th}
          class:active={theme.choice === th}
          onclick={() => theme.set(th as 'system'|'light'|'dark')}
          title={t(`app.themes.${th as 'system'|'light'|'dark'}`)}
          aria-label={t(`app.themes.${th as 'system'|'light'|'dark'}`)}
        >
          <Icon size={13}/>
        </button>
      {/each}
    </div>
    <button
      class="btn quiet icon"
      title={t('workspace.helpTitle')}
      aria-label={t('workspace.helpTitle')}
      onclick={(event) => {
        event.currentTarget.focus({ preventScroll: true });
        showHelp = true;
      }}
    >
      <CircleHelp size={16}/>
    </button>
    <button
      class="btn"
      class:primary={!!result}
      disabled={!result || busy || result.request_id !== current || !result.bodies.length}
      title={!result ? t('workspace.allRequired') : t('workspace.export')}
      onclick={exportStep}
    >
      <Download size={14}/>
      {t('workspace.export')}
    </button>
  </header>
  <input
    class="sr-only"
    bind:this={input}
    type="file"
    tabindex="-1"
    accept=".stl,.3mf,model/stl,application/sla,model/3mf"
    onchange={(e) => { const f = e.currentTarget.files?.[0]; if (f) void load(f); e.currentTarget.value = ''; }}
  />
  <main>
    <section class="workspace">
      <div class="workspace-head">
        <div class="view-switch" role="group" aria-label={t('stepInspection.view')}>
          <button
            class:active={!stepView && !stepPending}
            aria-pressed={!stepView && !stepPending}
            tabindex={!stepView && !stepPending ? 0 : -1}
            onkeydown={viewKey}
            onclick={showSource}
          >
            {t('stepInspection.source')}
          </button>
          <button
            class:active={stepView || stepPending}
            aria-pressed={stepView || stepPending}
            tabindex={stepView || stepPending ? 0 : -1}
            disabled={!result || result.request_id !== current}
            title={t('stepInspection.preview')}
            onkeydown={viewKey}
            onclick={showStepResult}
          >
            {t('stepInspection.result')}
          </button>
        </div>
        <div class="spacer"></div>
        <button class="btn quiet section-toggle" class:active={sectionActive} aria-pressed={sectionActive} title={t(stepView?'section.toggleHint':'sourceSection.toggleHint')} disabled={!canOpenSection()&&!sectionActive} onclick={toggleSection}><Scissors size={14}/>{t(stepView?'section.toggle':'sourceSection.toggle')}</button>
        <button
          class="btn quiet selection-toggle"
          class:active={selectionMode && (!largeTriangles || stepView)}
          aria-pressed={selectionMode && (!largeTriangles || stepView)}
          title={stepView ? t('stepInspection.toggleHint') : t('inspection.toggleHint')}
          disabled={!bytes || (!!largeTriangles && !stepView)}
          onclick={toggleSelection}
        >
          <MousePointer2 size={14}/>
          {t('inspection.toggle')}
        </button>
        <label class="edge-toggle" title={t('workspace.edges')}>
          <input type="checkbox" bind:checked={showEdges}/>
          {t('workspace.edges')}
        </label>
        <button class="btn quiet icon" title={t('workspace.fit')} aria-label={t('workspace.fit')} onclick={() => fitSignal++}>
          <Focus size={16}/>
        </button>
      </div>
      {#if stepView && regionError}<p class="muted" role="status">{regionError}</p>{/if}
      {#if stepView && stepDocument?.request_id === current && stepSelection?.faces.length}
        <GuidedCorrectionBar
          count={stepSelection.faces.length}
          canguide={!!eligibleGuidedFaces}
          canprepare={canPrepareGuidance}
          pending={stepSelectionPending || busy || !!processing}
          onguide={tryGuidedFamily}
          onprepare={() => change({reconstruction:'planar'})}
          onclear={clearStepSelection}
        />
      {/if}
      <div class="canvas">
        <Viewport
          label={bytes && (format === 'stl' || preview) ? (stepView ? t('stepInspection.preview') : t('workspace.preview')) : null}
          stepDisplay={stepView ? stepDisplay : null}
          sectionPlane={sectionActive?sectionDisplayFrame:null}
          {sectionBounds}
          sectionMode={sectionActive?sectionMode:null}
          onsectiontransform={transformSection}
          onsectiondrag={sectionDrag}
          {sourceRevision}
          selectionMode={selectionMode && (!largeTriangles || stepView)}
          zone={sourceZone}
          sourceHighlights={mappedSource && stepSelection?.source.status === 'exact' ? stepSelection.source.bodies : null}
          onpick={pickSourceTriangle}
          onmiss={clearVisibleSelection}
          stepPreview={stepView ? stepDocument : null}
          stepTriangles={stepSelection?.triangles ?? []}
          onfacepick={(id,additive) => stepCommand('pick',id,additive)}
          onfaceregion={selectStepRegion}
          onfacepaint={paintStepFaces}
          {bytes}
          results={selection ? [selection] : result?.bodies ?? []}
          {preview}
          {format}
          {fitSignal}
          {showEdges}
          unit={options.input_unit}
        />
        {#if !bytes || (format === '3mf' && !preview)}
          <div class="empty">
            <FileUp size={32}/>
            <strong>
              {bytes && format === '3mf' ? (!native() ? t('workspace.preview3mf') : processing ? t(`processing.${processing.phase}Title`) : error ? t(errorTitle) : t('workspace.waiting')) : t('workspace.drop')}
            </strong>
            <span>{bytes && format === '3mf' ? '' : t('workspace.guidance')}</span>
            {#if !bytes}
              <button class="btn" onclick={() => input.click()}>
                <FileUp size={14}/>
                {t('workspace.open')}
              </button>
            {/if}
          </div>
        {:else}
          <div class="canvas-label">
            {stepView ? t('stepInspection.preview') : mappedSource ? t('stepInspection.sourceZone') : t('workspace.preview')}
          </div>
          <div class="canvas-help">
            {selectionMode && (!largeTriangles || stepView) ? (stepView ? t('stepInspection.gestureHint') : t('inspection.gestureHint')) : t('workspace.orbit')}
          </div>
        {/if}
        {#if stepPending || stepUnavailable}
          <div class="step-preview-status" role="status">
            {stepPending ? t('stepInspection.pending') : t('stepInspection.unavailable')}
          </div>
        {/if}
        {#if processing}
          <ProcessingIndicator phase={processing.phase} startedAt={processing.startedAt} progress={conversionProgress}/>
        {/if}
      </div>
    </section>
    <aside class="panel inspector" aria-label={t('workspace.inspector')}>
      <div class="panel-title">{t('workspace.inspector')}</div>
      <div class="document-state" aria-live="polite">
        <strong class:success={!!result && !error} class:danger={!!error}>
          {error ? t(errorTitle) : processing ? t(`processing.${processing.phase}Title`) : result ? t('workspace.ready') : bytes && !native() ? t('workspace.native') : (preview || largeTriangles) ? t('workspace.sourceReady') : t('workspace.waiting')}
        </strong>
        {#if !result && !error && !processing}
          <p class="muted">{bytes ? t(native() ? 'workspace.reconstructHint' : 'workspace.nativeNote') : t('workspace.emptyNext')}</p>
        {/if}
        {#if largeTriangles}
          <p class="message warning"><TriangleAlert size={14}/><span>{t('workspace.largeWarning', { n: largeTriangles.toLocaleString(i18n.locale) })}</span></p>
          <p class="muted">{t('workspace.largeLimits')}</p>
        {/if}
        {#if largeTriangles && (processing || cancelling) && busy}
          <button class="btn" disabled={cancelling} onclick={() => workflow.cancelLarge()}>{t(cancelling ? 'workspace.largeCancelling' : 'workspace.largeCancel')}</button>
        {/if}
        {#if bytes && native() && (preview || largeTriangles) && !result && !guidedAttempt}
          <button class="btn primary" disabled={!canReconstruct} onclick={() => reconstruct(largeTriangles !== null)} title={t('workspace.reconstructHint')}>
            {t(largeTriangles ? 'workspace.largeTry' : 'workspace.reconstruct')}
          </button>
        {/if}
        {#if error}
          <p class="message danger">
            <TriangleAlert size={14}/>
            <span>
              {error}
              {#if recognitionFailed}
                <small>{t('workspace.strictError')}</small>
              {/if}
            </span>
          </p>
        {/if}
        {#if guidedRefusal}
          <p class="muted">{t('guidance.refusalRecovery')}</p>
          <button class="btn" disabled={busy || !!processing} onclick={recomputeGuidance}>
            {t(guidanceHints.length ? 'guidance.recompute' : 'guidance.recomputeBase')}
          </button>
        {/if}
        {#if bytes && !native()}
          <p class="message warning">
            <TriangleAlert size={14}/>
            <span>{format === '3mf' ? t('workspace.preview3mf') : t('workspace.nativeNote')}</span>
          </p>
        {/if}
        {#each notices.filter(item => visibleDiagnostic(item.diagnostic)) as item}
          <p class="message" class:warning={item.diagnostic.severity === 'warning'}>
            <TriangleAlert size={14}/>
            <span>
              {#if item.body}
                <small>{item.index + 1} · {bodyText(item.body)}</small>
              {/if}
              {diagnosticText(item.diagnostic.code, item.diagnostic.message, item.body)}
              ({item.diagnostic.count})
            </span>
          </p>
        {/each}
      </div>
      {#if canPrepareGuidance && !(stepView && stepSelection?.faces.length)}
        <div class="document-controls">
          <button class="btn" onclick={() => change({reconstruction:'planar'})} title={t('guidance.prepareHint')}>{t('guidance.prepare')}</button>
          <p class="muted">{t('guidance.prepareHint')}</p>
        </div>
      {/if}
      {#if sectionActive}
        <SectionTool input={sectionInput} faceId={sectionTarget==='step'?selectedPlaneFace():null} solids={sectionSolids} state={sectionState} canexport={sections.canExport()} onchange={changeSection} onclose={closeSection} onexport={sections.exportDxf} {errorText} mode={sectionMode} manipulating={sectionDragging} onmodechange={mode=>sectionMode=mode} onreset={resetSectionPlane} target={sectionTarget} sourcePreparation={sourcePreparationState} bodyLabel={sectionBodyName} diagnosticLabel={diagnostic=>diagnosticText(diagnostic.code,diagnostic.message,null)}/>
      {:else if stepView && stepDocument && stepSelection?.faces.length}
        <StepInspection
          selection={stepSelection}
          document={stepDocument}
          revealContext={stepContextReveal}
          ongrow={() => stepCommand('grow')}
          onshrink={() => stepCommand('shrink')}
          onclear={clearStepSelection}
          onshowsource={showMappedSource}
          profile={faceProfile}
          {profileEligible}
          canexportprofile={faceProfiles.canExport()}
          onexportprofile={faceProfiles.exportDxf}
          {errorText}
        />
      {:else if mappedSource && stepSelection?.source.status === 'exact'}
        <StepSourceInspection selection={stepSelection} onback={showStepResult} onclear={clearStepSelection}/>
      {:else if !stepView && sourceZone}
        <SourceInspection
          zone={sourceZone}
          inspection={zoneInspection}
          bodyName={preview?.bodies.find(body => body.id === sourceZone?.previewBodyId) ? bodyText(preview.bodies.find(body => body.id === sourceZone?.previewBodyId)!) : t('workspace.stlBody')}
          {topologyState}
          {hasMore}
          ongrow={() => resizeSourceZone(1)}
          onshrink={() => resizeSourceZone(-1)}
          onclear={clearSourceZone}
        />
      {:else if bytes && !stepView && (result || selectionMode)}
        <div class="context-hint">
          <MousePointer2 size={14}/>
          <p class="muted">
            {selectionMode ? t('inspection.pickHint') : t('workspace.inspectHint')}
          </p>
        </div>
      {/if}
      {#if guidanceHints.length}
        <Section id="guided-attempt" title={t('guidance.title')}>
          {#if guidedAttempt}
            <p class="muted" role="status">{t(`guidance.${guidedAttempt.status}`)}</p>
          {/if}
          <div class="metric">
            <span>{t('guidance.accepted')}</span>
            <strong>{guidanceHints.length} / 16</strong>
          </div>
          {#each guidanceHints as hint (hint.hint_id)}
            <div class="guidance-hint">
              <strong>{t(guidedFamilyLabel(hint.family), {n:hint.hint_id})}</strong>
              <Disclosure id={`guidance-trace-${hint.hint_id}`} label={t('workspace.details')}>
                <p class="muted">
                  {(preview?.bodies.findIndex(body => body.id === hint.preview_body_id) ?? -1) + 1}
                  ·
                  {bodyText((result?.bodies ?? preview?.bodies ?? []).find(body => body.id === hint.analysis_body_id || body.id === hint.preview_body_id) ?? {id:hint.preview_body_id,name:hint.preview_body_id})}
                </p>
                <p class="muted">{t('guidance.selected', {n:hint.selected_face_ids.length})}</p>
                <div class="metric">
                  <span>{t('guidance.trace')}</span>
                  <strong>{hint.selected_source_triangle_ids.length} / {hint.reconstructed_source_triangle_ids.length}</strong>
                </div>
              </Disclosure>
              <button
                class="btn"
                disabled={busy || !!processing}
                aria-label={t('guidance.removeNamed', {n:hint.hint_id})}
                onclick={() => removeGuidance(hint.hint_id)}
              >
                {t('guidance.remove')}
              </button>
            </div>
          {/each}
          {#if guidanceHints.length === 16}
            <p class="muted">{t('guidance.limit')}</p>
          {/if}
        </Section>
      {/if}
      <div class="document-controls">
        {#if guidanceHints.length || guidedAttempt}
          <p class="message warning" role="note">{t('guidance.optionsResetWarning')}</p>
        {/if}
        {#if format === '3mf'}
          <div class="property" title={t('workspace.unitLocked')}>
            <span>
              {t('workspace.unit3mf')}
              <span class="hint">ⓘ</span>
            </span>
            <strong>
              {(preview ? unitText(preview.source.unit) : null) ?? (processing?.phase === 'preview' ? t('workspace.unitReading') : t('workspace.unitUnknown'))}
            </strong>
          </div>
          <p class="muted">
            {preview ? (preview.source.unit_origin === 'default' ? t('workspace.unitDefault') : t('workspace.unitDeclared')) : t('workspace.unitLocked')}
          </p>
        {:else}
          <label class="property" title={t('workspace.unitHint')}>
            <span>
              {t('workspace.unit')}
              <span class="hint">ⓘ</span>
            </span>
            <select class="input" value={options.input_unit} onchange={(e) => change({ input_unit:e.currentTarget.value as 'mm'|'inch' })}>
              <option value="mm">{t('workspace.mm')} (mm)</option>
              <option value="inch">{t('workspace.inch')} (in)</option>
            </select>
          </label>
        {/if}
        <Disclosure id="ux4-advanced" label={t('workspace.advanced')}>
          <label class="property mode-property">
            <span>{t('workspace.mode')}</span>
            <select
              class="input"
              value={options.reconstruction}
              onchange={(e) => change({ reconstruction:e.currentTarget.value as Options['reconstruction'] })}
            >
              <option value="auto">{t('workspace.autoChoice')}</option>
              <option value="planar">{t('workspace.planarChoice')}</option>
              <option value="surfaces">{t('workspace.surfacesChoice')}</option>
              <option value="cylinders">{t('workspace.cylinderChoice')}</option>
              <option value="primitives">{t('workspace.primitivesMode')}</option>
              <option value="height_patch">{t('workspace.heightPatchChoice')}</option>
            </select>
          </label>
          <p class="muted">{options.reconstruction === 'auto' ? t('workspace.autoIntent') : options.reconstruction === 'height_patch' ? t('workspace.heightPatchIntent') : t('workspace.manualIntent')}</p>
          {@render toleranceField('weld_tolerance_mm', 'workspace.weld', 'workspace.weldHint', 0.000001)}
          {@render toleranceField('plane_tolerance_mm', 'workspace.plane', 'workspace.planeHint', 0.000001)}
          {#if options.reconstruction !== 'planar' || stepView || !!guidedAttempt || !validPreferences(options)}
            {#if options.reconstruction === 'height_patch'}
              {@render toleranceField('cylinder_fit_tolerance_mm', 'workspace.heightTolerance', 'workspace.heightToleranceHint', 0.000001)}
            {:else}
              {@render toleranceField('cylinder_fit_tolerance_mm', 'workspace.fitTolerance', 'workspace.fitToleranceHint', 0.000001)}
            {/if}
            {@render toleranceField('max_deviation_mm', 'workspace.maxDeviation', 'workspace.maxDeviationHint', 0.01)}
            {#if options.reconstruction !== 'height_patch'}<p class="muted">{t('workspace.toleranceExplanation')}</p>{/if}
          {/if}
          <Disclosure id="reconstruction-limits" label={t('workspace.reconstructionLimits')}>
            <p class="muted">{t('workspace.brepLimit')}</p>
            <p class="muted">
              {options.reconstruction === 'auto' ? t('workspace.autoIntent') : options.reconstruction === 'height_patch' ? t('workspace.heightPatchLimit') : options.reconstruction === 'surfaces' ? t('workspace.surfacesLimit') : options.reconstruction === 'primitives' ? t('workspace.primitivesLimit') : options.reconstruction === 'cylinders' ? t('workspace.cylinderLimit') : t('workspace.limit')}
            </p>
            {#if options.reconstruction !== 'planar' && options.reconstruction !== 'auto'}
              <p class="muted">{t('workspace.recognitionRefusal')}</p>
            {/if}
          </Disclosure>
          {#if allowParameterSearch && !recognitionFailed}
            <ParameterSearch
              enabled={canSearch}
              searching={processing?.phase === 'searching'}
              report={parameterProposal?.response.report ?? null}
              currentTolerance={options.cylinder_fit_tolerance_mm}
              error={parameterError ? errorText(parameterError) : ''}
              {errorText}
              onsearch={searchParameters}
              onapply={applyParameterProposal}
            />
          {/if}
        </Disclosure>
        {#if allowParameterSearch && recognitionFailed}
          <ParameterSearch
            enabled={canSearch}
            searching={processing?.phase === 'searching'}
            report={parameterProposal?.response.report ?? null}
            currentTolerance={options.cylinder_fit_tolerance_mm}
            error={parameterError ? errorText(parameterError) : ''}
            {errorText}
            onsearch={searchParameters}
            onapply={applyParameterProposal}
          />
        {/if}
      </div>
      {#if !stepView && !mappedSource && (result?.bodies.length ?? preview?.bodies.length ?? 0) > 1}
        <div class="body-selector">
          <label class="property">
            <span>{t('workspace.body')}</span>
            <select class="input" value={selectedId} onchange={(e) => selectedId=e.currentTarget.value}>
              <option value="">{t('workspace.allBodies')}</option>
              {#each result?.bodies ?? preview?.bodies ?? [] as body, index}
                <option value={body.id}>{index + 1} · {bodyText(body)}</option>
              {/each}
            </select>
          </label>
          <div class="metric">
            <span>{result ? t('workspace.bodies') : t('workspace.previewCount')}</span>
            <strong>{result?.bodies.length ?? preview?.bodies.length ?? '—'}</strong>
          </div>
        </div>
      {/if}
      <Section id="workshop-result-details" title={t('workspace.resultDetails')} open={false}>
        <Section id="mesh" title={t('workspace.mesh')}>
          <div class="metric">
            <span>{t('workspace.triangles')}</span>
            <strong>{sourceTriangles != null ? fmt(sourceTriangles) : '—'}</strong>
          </div>
          <div class="metric">
            <span>{t('workspace.vertices')}</span>
            <strong>{sourceVertices != null ? fmt(sourceVertices) : '—'}</strong>
          </div>
        </Section>
        <Section id="solid" title={t('workspace.result')}>
          <div class="metric">
            <span>{t('workspace.faces')}</span>
            <strong>{metrics ? fmt(metrics.faces) : '—'}</strong>
          </div>
          <div class="metric">
            <span>{t('workspace.planeFaces')}</span>
            <strong>{metrics ? fmt(metrics.planar_faces) : '—'}</strong>
          </div>
          <div class="metric">
            <span>{t('workspace.cylinderFaces')}</span>
            <strong>{metrics ? fmt(metrics.cylindrical_faces) : '—'}</strong>
          </div>
          <div class="metric">
            <span>{t('workspace.conicalFaces')}</span>
            <strong>{metrics ? fmt(metrics.conical_faces) : '—'}</strong>
          </div>
          <div class="metric">
            <span>{t('workspace.sphericalFaces')}</span>
            <strong>{metrics ? fmt(metrics.spherical_faces) : '—'}</strong>
          </div>
          <div class="metric">
            <span>{t('workspace.toroidalFaces')}</span>
            <strong>{metrics ? fmt(metrics.toroidal_faces) : '—'}</strong>
          </div>
          {#if metrics?.revolution_faces}
            <div class="metric">
              <span>{t('inspection.revolution')}</span>
              <strong>{fmt(metrics.revolution_faces)}</strong>
            </div>
          {/if}
          {#if heightPatches.length}
            <div class="metric">
              <span>{t('inspection.heightPatch')}</span>
              <strong>{fmt(heightPatches.length)}</strong>
            </div>
          {/if}
          {#if selection?.rounded_recognition}
            <div class="metric">
              <span>{t('workspace.toroidalFillets')}</span>
              <strong>{fmt(selection.rounded_recognition.report.toroidal_fillets)}</strong>
            </div>
          {/if}
          {#if report}
            <div class="metric" title={t('workspace.radialResidualHint')}>
              <span>{t('workspace.radialResidual')}</span>
              <strong>{measuredValue(report.max_radial_residual_mm)} mm</strong>
            </div>
          {/if}
          {#if report || heightBound !== null}
            <div class="metric" title={t('workspace.maxDeviationHint')}>
              <span>{t('workspace.deviation')}</span>
              <strong>{precise(report?.max_deviation_bound_mm ?? heightBound!)} mm</strong>
            </div>
          {/if}
          <div class="metric">
            <span>{t('workspace.solids')}</span>
            <strong>{metrics ? fmt(metrics.solids) : '—'}</strong>
          </div>
          <div class="metric">
            <span>{t('workspace.volume')}</span>
            <strong>{metrics ? `${fmt(metrics.volume_mm3)} mm³` : '—'}</strong>
          </div>
          <div class="metric">
            <span>{t('workspace.dimensions')}</span>
            <strong>{dims}{metrics ? ' mm' : ''}</strong>
          </div>
          {#if selection?.fidelity}
            <Disclosure id="quality" label={t('workspace.quality')}>
              <div class="metric">
                <span>{t('workspace.sourceToResult')}</span>
                <strong>{precise(selection.fidelity.source_to_result_mm)} mm</strong>
              </div>
              <p class="muted">{fmt(selection.fidelity.source_samples)} {t('workspace.samples')}</p>
              <div class="metric">
                <span>{t('workspace.resultToSource')}</span>
                <strong>{precise(selection.fidelity.result_to_source_mm)} mm</strong>
              </div>
              <p class="muted">{fmt(selection.fidelity.result_samples)} {t('workspace.samples')}</p>
              <div class="metric">
                <span>{t('workspace.deflection')}</span>
                <strong>{precise(selection.fidelity.tessellation_deflection_mm)} mm</strong>
              </div>
              <p class="muted">{t('workspace.qualityCaveat')}</p>
            </Disclosure>
          {/if}
          {#if result}
            <Disclosure id="applied-settings" label={t('workspace.appliedSettings')}>
              <div class="metric">
                <span>{t('workspace.mode')}</span>
                <strong>
                  {!!result.guidance_hints.length ? t('guidance.title') : result.settings.reconstruction === 'auto' ? t('workspace.autoChoice') : result.settings.reconstruction === 'height_patch' ? t('workspace.heightPatchChoice') : result.settings.reconstruction === 'surfaces' ? t('workspace.surfacesMode') : result.settings.reconstruction === 'primitives' ? t('workspace.primitivesMode') : result.settings.reconstruction === 'cylinders' ? t('workspace.cylinderMode') : t('workspace.planarMode')}
                </strong>
              </div>
              <div class="metric">
                <span>{t('workspace.weld')}</span>
                <strong>{settingValue(result.settings.weld_tolerance_mm)} mm</strong>
              </div>
              <div class="metric">
                <span>{t('workspace.plane')}</span>
                <strong>{settingValue(result.settings.plane_tolerance_mm)} mm</strong>
              </div>
              {#if result.settings.reconstruction !== 'planar' || !!result.guidance_hints.length}
                <div class="metric">
                  <span>{t(result.settings.reconstruction === 'height_patch' ? 'workspace.heightTolerance' : 'workspace.fitTolerance')}</span>
                  <strong>{settingValue(result.settings.cylinder_fit_tolerance_mm)} mm</strong>
                </div>
                <div class="metric">
                  <span>{t('workspace.maxDeviation')}</span>
                  <strong>{settingValue(result.settings.max_deviation_mm)} mm</strong>
                </div>
              {/if}
            </Disclosure>
          {/if}
          {#if !result}
            <p class="muted">{t('workspace.noResult')}</p>
          {/if}
        </Section>
        <Section id="diagnostics" title={t('workspace.diagnostics')}>
          {#each notices as item}
            <p class:warning={item.diagnostic.severity === 'warning'} class="message">
              <TriangleAlert size={14}/>
              <span>
                {#if item.body}
                  <small>{item.index + 1} · {bodyText(item.body)}</small>
                {/if}
                {diagnosticText(item.diagnostic.code, item.diagnostic.message, item.body)}
                ({item.diagnostic.count})
                <small>{item.diagnostic.code}</small>
              </span>
            </p>
          {/each}
        </Section>
      </Section>
    </aside>
  </main>
  {#if showHelp}
    <Dialog title={t('workspace.helpTitle')} onclose={() => showHelp = false}>
      <h3>{t('workspace.helpAbout')}</h3>
      <p class="help-copy">{t('workspace.helpVersion', {version:packageInfo.version})}</p>
      <h3>{t('workspace.helpUsage')}</h3>
      <p class="help-copy">{t('workspace.helpUnits')}</p>
      <p class="help-copy">{t('workspace.helpLimits')}</p>
      <p class="help-copy">{t('workspace.helpPreview')}</p>
      <h3>{t('workspace.helpShortcuts')}</h3>
      <p class="help-copy">{t('workspace.helpKeys')}</p>
    </Dialog>
  {/if}
  <footer>
    <span class:ok={!!result} class:warn={!!processing || (!!bytes && !native())} class:bad={!!error} class="dot"></span>
    <span>
      {error ? t(errorTitle) : processing ? t('workspace.pending') : result ? t('workspace.ready') : bytes && !native() ? t('workspace.native') : (preview || largeTriangles) ? t('workspace.sourceReady') : t('workspace.waiting')}
    </span>
    <span class="spacer"></span>
    <span>{noteKey ? t(noteKey) : (result ? result.cad.kernel : 'FabSTEP')}</span>
  </footer>
  {#if dragging}
    <div class="drop-overlay">{t('workspace.drop')}</div>
  {/if}
</div>
{#snippet toleranceField(field: Tolerance, label: Key, hint: Key, step: number)}
  <NumField
    label={t(label)}
    hint={t(hint)}
    value={options[field]}
    kind="plain"
    unit="mm"
    decimals="exact"
    {step}
    invalid={!Number.isFinite(options[field]) || options[field] <= 0}
    onchange={(value) => changeTolerance(field, value)}
    onbegin={() => { scrubbedTolerance = field; }}
    onend={finishToleranceScrub}
  />
{/snippet}

<style>
  .app {
    height: 100%;
    display: flex;
    flex-direction: column;
  }
  .top {
    min-height: var(--workshop-bar-height);
    flex: none;
    display: flex;
    align-items: center;
    gap: var(--s2);
    padding: var(--s1) var(--s3);
    background: var(--bg-chrome);
    border-top: 2px solid var(--identity);
    border-bottom: 1px solid var(--border);
  }
  .brand {
    display: flex;
    align-items: center;
    gap: var(--s2);
    white-space: nowrap;
    margin-right: var(--s3);
  }
  .brand strong {
    font-size: var(--fs-lg);
  }
  .brand span, .filename {
    color: var(--text-muted);
    font-size: var(--fs-sm);
  }
  .filename {
    min-width: 0;
    max-width: var(--workshop-document-name);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .spacer {
    flex: 1;
  }
  .switch {
    display: flex;
    gap: var(--s1);
    padding: 0;
    background: var(--segment-track);
    border-radius: var(--radius);
    height: var(--control-h);
  }
  .switch button {
    border: 0;
    background: none;
    color: var(--text-muted);
    padding-inline: var(--s1);
    font-size: var(--fs-xs);
    display: grid;
    place-items: center;
    border-radius: var(--radius);
  }
  .switch button.active {
    background: var(--bg-elevated);
    color: var(--text-primary);
  }
  main {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: minmax(var(--workshop-min-canvas), 1fr) var(--workshop-inspector-width);
  }
  .panel {
    background: var(--bg-panel);
    overflow: auto;
    min-width: 0;
  }
  .inspector {
    border-left: 1px solid var(--border);
  }
  .panel-title {
    min-height: var(--control-h);
    display: flex;
    align-items: center;
    padding: var(--s2) var(--s3);
    border-bottom: 1px solid var(--divider);
    color: var(--text-section);
    font-size: var(--fs-xs);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: .06em;
  }
  .document-state, .document-controls, .body-selector, .context-hint {
    padding: var(--s3);
    display: flex;
    flex-direction: column;
    gap: var(--s2);
    border-bottom: 1px solid var(--divider);
  }
  .document-state > strong {
    font-size: var(--fs-md);
    font-weight: 600;
  }
  .success {
    color: var(--success);
  }
  .danger {
    color: var(--danger-text);
  }
  .context-hint {
    flex-direction: row;
    align-items: flex-start;
    color: var(--text-muted);
  }
  .context-hint :global(svg) {
    flex: none;
  }
  .workspace {
    display: flex;
    flex-direction: column;
    min-width: 0;
    background: var(--bg-workspace);
  }
  .workspace-head {
    display: flex;
    align-items: center;
    gap: var(--s2);
    flex-wrap: wrap;
    padding: var(--s1) var(--s2);
    border-bottom: 1px solid var(--divider);
    color: var(--text-secondary);
    font-size: var(--fs-xs);
  }
  .view-switch {
    display: flex;
    gap: var(--s1);
    padding: 0;
    height: var(--control-h);
    background: var(--segment-track);
    border-radius: var(--radius);
  }
  .view-switch button {
    border: 0;
    border-radius: var(--radius);
    padding-inline: var(--s2);
    background: transparent;
    color: var(--text-muted);
    font-size: var(--fs-xs);
    white-space: nowrap;
  }
  .view-switch button.active {
    background: var(--bg-elevated);
    color: var(--text-primary);
  }
  .view-switch button:disabled {
    opacity: .5;
    cursor: default;
  }
  .selection-toggle,.section-toggle {
    font-size: var(--fs-xs);
  }
  .selection-toggle.active,.section-toggle.active {
    background: var(--bg-selected);
    color: var(--accent-text);
    border-color: var(--accent);
  }
  .canvas {
    position: relative;
    flex: 1;
    min-height: 0;
  }
  .empty {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-direction: column;
    gap: var(--s2);
    padding: var(--s4);
    pointer-events: none;
    color: var(--text-muted);
    text-align: center;
    font-size: var(--fs-sm);
  }
  .empty strong {
    color: var(--text-secondary);
    font-size: var(--fs-lg);
    font-weight: 500;
  }
  .empty .btn {
    pointer-events: auto;
    margin-top: var(--s2);
  }
  .canvas-label, .canvas-help {
    position: absolute;
    pointer-events: none;
    color: var(--text-muted);
    font-size: var(--fs-xs);
    max-width: calc(100% - var(--s4) * 2);
  }
  .canvas-label {
    left: var(--s4);
    top: var(--s3);
  }
  .canvas-help {
    left: var(--s4);
    bottom: var(--s3);
  }
  .step-preview-status {
    position: absolute;
    top: var(--s4);
    right: var(--s4);
    max-width: 60%;
    padding: var(--s2);
    background: var(--bg-panel);
    color: var(--text-secondary);
    font-size: var(--fs-xs);
    border: 1px solid var(--divider);
    border-radius: var(--radius);
  }
  .edge-toggle {
    display: flex;
    align-items: center;
    gap: var(--s1);
    white-space: nowrap;
  }
  .edge-toggle input {
    accent-color: var(--accent);
  }
  .property, .metric {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--s2);
    min-height: var(--control-h);
    color: var(--text-secondary);
    font-size: var(--fs-sm);
  }
  .property > span:first-child {
    min-width: 0;
  }
  .property select {
    width: var(--workshop-field-width);
    font-size: var(--fs-sm);
  }
  .mode-property {
    align-items: stretch;
    flex-direction: column;
    gap: var(--s1);
  }
  .mode-property select {
    width: 100%;
  }
  .muted {
    color: var(--text-muted);
    font-size: var(--fs-xs);
    line-height: 1.5;
    margin: 0;
  }
  .metric strong {
    font-weight: 500;
    color: var(--text-primary);
    text-align: right;
    font-variant-numeric: tabular-nums;
    overflow-wrap: anywhere;
    min-width: 0;
  }
  .message {
    display: flex;
    gap: var(--s2);
    align-items: flex-start;
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--fs-sm);
    overflow-wrap: anywhere;
  }
  .message :global(svg) {
    flex: none;
  }
  .message small {
    display: block;
    color: var(--text-muted);
  }
  .message.warning {
    color: var(--warning);
  }
  .message.danger {
    color: var(--danger-text);
  }
  .guidance-hint {
    display: grid;
    gap: var(--s1);
    padding-block: var(--s2);
    border-top: 1px solid var(--border);
    font-size: var(--fs-sm);
  }
  .help-copy {
    margin: 0;
  }
  footer {
    height: var(--workshop-status-height);
    flex: none;
    display: flex;
    align-items: center;
    gap: var(--s2);
    padding-inline: var(--s3);
    background: var(--bg-chrome);
    border-top: 1px solid var(--border);
    color: var(--text-muted);
    font-size: var(--fs-xs);
  }
  .dot {
    height: var(--s2);
    width: var(--s2);
    background: var(--text-muted);
    border-radius: 50%;
    flex: none;
  }
  .dot.ok {
    background: var(--success);
  }
  .dot.warn {
    background: var(--warning);
  }
  .dot.bad {
    background: var(--danger);
  }
  .drop-overlay {
    position: fixed;
    inset: var(--s2);
    display: grid;
    place-items: center;
    border: 2px dashed var(--accent);
    background: var(--accent-tint);
    color: var(--accent-text);
    font-size: var(--fs-lg);
    pointer-events: none;
  }
  @media (max-width: 1100px) {
    main {
      grid-template-columns: minmax(var(--workshop-min-canvas), 1fr) var(--workshop-inspector-compact);
    }
    .brand span {
      display: none;
    }
    .filename {
      flex: 1;
    }
  }
  @media (max-width: 760px) {
    .top {
      flex-wrap: wrap;
    }
    .brand {
      margin-right: 0;
    }
    .filename {
      display: none;
    }
    main {
      grid-template-columns: minmax(0, 1fr);
      overflow: auto;
    }
    .workspace {
      min-height: var(--workshop-small-canvas-height);
    }
    .inspector {
      border-left: 0;
      border-top: 1px solid var(--border);
      overflow: visible;
    }
  }
</style>
