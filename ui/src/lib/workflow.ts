import { largeStlTriangles } from './large-stl.ts';
import { validPreferences } from './editing.ts';
import { acceptsGuidedResult } from './guided.ts';
import { uploadSource, type SourceBridge } from './source.ts';
import type { Key } from './i18n.svelte';
import type { ConversionResult, DocumentPreview, GuidanceHistory, GuidedFamily, GuidedReport, Options, Result } from './types.ts';
export type WorkflowState = {
  largeTriangles: number | null;
  largeAccepted: boolean;
  cancelling: boolean;
  options: Options;
  bytes: Uint8Array | null;
  name: string;
  format: 'stl' | '3mf';
  current: number;
  fileToken: number;
  sourceId: number | null;
  uploadedSourceId: number | null;
  sourceRevision: number;
  result: ConversionResult | null;
  preview: DocumentPreview | null;
  selectedId: string;
  errorState: {
    kind: 'key';
    key: Key;
  } | {
    kind: 'native';
    value: unknown;
  } | null;
  noteKey: Key | null;
  recognitionFailed: boolean;
  busy: boolean;
  queued: boolean;
  processing: {
    requestId: number;
    phase: 'queued' | 'preview' | 'converting' | 'searching';
    startedAt: number;
  } | null;
  topologyState: 'pending' | 'ready' | 'unavailable';
  guidanceHints: GuidedReport[];
  guidanceHistory?: GuidanceHistory;
  guidedAttempt: {
    requestId: number;
    status: 'pending' | 'success' | 'refused';
  } | null;
  fitSignal: number;
};
export type WorkflowDependencies = {
  native: () => SourceBridge | undefined;
  serializableOptions: (options: Options) => Options;
  clearParameterSearch: () => void;
  resetSourceInspection: (newGeometry: boolean) => void;
  resetStepInspection: () => void;
  prepareSelection: (id: number, key: number, options: Options, preview: DocumentPreview, revision: number) => void | Promise<void>;
  interpretSourceZone: () => void;
  onResultReady: () => void;
  now: () => number;
  schedule: (callback: () => void, delay: number) => unknown;
  clearSchedule: (handle: unknown) => void;
};
/** The supplied facade reads/writes App's live state, including its shared engine lock. */
export function createWorkflow(state: WorkflowState, deps: WorkflowDependencies) {
  let timer: unknown;
  let disposed = false;
  // Opening a source prepares its preview. Conversion starts only after an
  // explicit request for this source, then options retain their live recalculation.
  let reconstructionRequested = false;
  let ordinaryAutoId: number | null = null;

  function clearTimer() {
    if (timer !== undefined) {
      deps.clearSchedule(timer);
    }
    timer = undefined;
  }

  function invalidate(preservePreview = false) {
    if (disposed) {
      return;
    }
    // An accepted/restored STEP or guided session also establishes that this
    // source has already entered the reconstruction workflow.
    reconstructionRequested ||= !!state.result || !!state.guidedAttempt || !!state.guidanceHints.length;
    deps.clearParameterSearch();
    state.guidedAttempt = null;
    state.guidanceHints = [];
    state.guidanceHistory = undefined;
    state.current++;
    deps.resetSourceInspection(!preservePreview);
    deps.resetStepInspection();
    const bridge = deps.native();
    if (bridge) {
      void bridge.invoke<void>('invalidate_result', { requestId: state.current, sourceId: state.sourceId }).catch(() => { /* Invalidation is best effort; epoch guards still reject old replies. */ });
    }
    state.result = null;
    if (!preservePreview) {
      state.preview = null;
      state.selectedId = '';
    }
    state.errorState = null;
    state.noteKey = null;
    state.recognitionFailed = false;
    clearTimer();
    state.queued = false;
    state.processing = null;
    if (!validPreferences(state.options)) {
      state.topologyState = 'unavailable';
      state.errorState = { kind: 'key', key: 'workspace.invalid' };
      return;
    }
    if (!state.bytes || !bridge) {
      return;
    }
    const id = state.current;
    state.processing = { requestId: id, phase: 'queued', startedAt: deps.now() };
    timer = deps.schedule(() => {
      timer = undefined;
      if (!disposed && id === state.current) {
        state.queued = true;
        void run();
      }
    }, 250);
  }

  async function run() {
    const bridge = deps.native();
    if (disposed || state.busy || !state.queued || !state.bytes || state.sourceId === null || !bridge) {
      return;
    }
    state.queued = false;
    state.busy = true;
    const id = state.current, source = state.bytes, key = state.sourceId;
    const opts = deps.serializableOptions(state.options), format = state.format;
    const shouldConvert = reconstructionRequested;
    const large = state.largeTriangles !== null;
    if (state.processing?.requestId === id) {
      state.processing.phase = 'preview';
    }
    let previewComplete = false;
    try {
      if (large && (!shouldConvert || !state.largeAccepted)) return;
      if (state.uploadedSourceId !== key) {
        await uploadSource(bridge, source, key, format);
        if (!disposed && state.sourceId === key) {
          state.uploadedSourceId = key;
        }
      }
      if (disposed || id !== state.current || state.sourceId !== key) {
        return;
      }
      if (!large) {
        const doc = await bridge.invoke<DocumentPreview>('preview_document', { requestId: id, sourceId: key, options: opts });
        if (disposed || id !== state.current || doc.request_id !== id) {
          return;
        }
        state.preview = doc;
        previewComplete = true;
        void deps.prepareSelection(id, key, opts, doc, state.sourceRevision);
      } else {
        state.topologyState = 'unavailable';
      }
      if (!shouldConvert) {
        return;
      }
      if (state.processing?.requestId === id) {
        state.processing.phase = 'converting';
      }
      ordinaryAutoId = !large && opts.reconstruction === 'auto' ? id : null;
      const converted = await bridge.invoke<ConversionResult>(large ? 'convert_large_document' : 'convert_document', { requestId: id, sourceId: key, options: opts, ...(large ? { confirmed: true } : {}) });
      if (!disposed && id === state.current && converted.request_id === id) {
        state.selectedId = '';
        state.result = converted;
        deps.interpretSourceZone();
        deps.onResultReady();
      }
    }
    catch (value) {
      if (!disposed && id === state.current) {
        if (!previewComplete) {
          state.topologyState = 'unavailable';
        }
        state.errorState = { kind: 'native', value };
        state.recognitionFailed = previewComplete
          && opts.reconstruction !== 'planar'
          && typeof value === 'object' && value !== null
          && 'code' in value && String(value.code).startsWith('recognition.');
      }
    }
    finally {
      ordinaryAutoId = null;
      if (state.processing?.requestId === id) {
        state.processing = null;
      }
      state.busy = false;
      if (state.queued && !disposed) {
        void run();
      }
    }
  }

  async function load(file: Pick<File, 'name' | 'size' | 'arrayBuffer'>) {
    if (disposed) {
      return;
    }
    const token = ++state.fileToken;
    resetReconstruction();
    state.bytes = null;
    state.sourceId = null;
    state.uploadedSourceId = null;
    state.name = file.name;
    invalidate();
    if (!/\.(stl|3mf)$/i.test(file.name)) {
      state.errorState = { kind: 'key', key: 'workspace.formatHint' };
      return;
    }
    state.format = /\.3mf$/i.test(file.name) ? '3mf' : 'stl';
    if (file.size > 64 * 1024 * 1024) {
      state.errorState = { kind: 'key', key: 'workspace.tooLarge' };
      return;
    }
    try {
      const data = new Uint8Array(await file.arrayBuffer());
      if (disposed || token !== state.fileToken) {
        return;
      }
      setSource(data);
      state.fitSignal++;
    }
    catch {
      if (!disposed && token === state.fileToken) {
        state.errorState = { kind: 'key', key: 'workspace.errorFile' };
      }
    }
  }

  function setSource(data: Uint8Array) {
    if (disposed) {
      return;
    }
    resetReconstruction();
    state.bytes = data;
    state.largeTriangles = largeStlTriangles(data, state.format);
    state.sourceId = state.current + 1;
    state.uploadedSourceId = null;
    invalidate();
  }

  function resetReconstruction() {
    reconstructionRequested = false;
    state.largeTriangles = null;
    state.largeAccepted = false;
    state.result = null;
    state.guidedAttempt = null;
    state.guidanceHints = [];
    state.guidanceHistory = undefined;
  }

  function canReconstruct() {
    return !disposed && !!deps.native() && !!state.bytes && state.sourceId !== null
      && !state.busy && !state.cancelling && (!state.processing || state.processing.phase === 'queued')
      && validPreferences(state.options);
  }

  async function reconstruct(acceptLarge = false) {
    if (!canReconstruct()) {
      return;
    }
    if (state.largeTriangles !== null) {
      if (!acceptLarge && !state.largeAccepted) return;
      state.largeAccepted = true;
    }
    reconstructionRequested = true;
    invalidate(true);
    // The explicit click starts immediately, replacing a pending preview timer.
    clearTimer();
    state.queued = true;
    await run();
  }

  function canCancelAuto() {
    return !disposed && state.busy && ordinaryAutoId === state.current;
  }

  function cancelAuto() {
    if (!canCancelAuto()) return;
    reconstructionRequested = false;
    invalidate(true);
    state.noteKey = 'workspace.cancelled';
  }

  async function cancelLarge() {
    const bridge = deps.native();
    if (!bridge || !state.largeTriangles || !state.busy || state.cancelling || state.sourceId === null) return;
    const requestId = state.current, sourceId = state.sourceId;
    state.cancelling = true;
    reconstructionRequested = false;
    clearTimer();
    state.queued = false;
    state.current++;
    const cancelledRevision = state.current;
    state.result = null;
    deps.resetStepInspection();
    try {
      await bridge.invoke<void>('cancel_large_conversion', { requestId, sourceId });
      if (!disposed && state.current === cancelledRevision) {
        state.processing = null;
        state.errorState = null;
        state.noteKey = 'workspace.cancelled';
      }
    } catch (value) {
      if (!disposed && state.current === cancelledRevision) state.errorState = { kind: 'native', value };
    } finally {
      state.cancelling = false;
    }
  }

  async function mutateGuidance(command: 'guided_convert' | 'remove_guidance' | 'edit_guidance' | 'undo_guidance' | 'redo_guidance' | 'recompute_guidance' | 'apply_similar_guided_regions', payload: {
    searchId?: number;
    candidateIds?: number[];
    faceIds?: number[];
    family?: GuidedFamily;
    hintId?: number;
  } = {}) {
    const bridge = deps.native();
    if (disposed || state.busy || state.processing || state.sourceId === null || !bridge || !state.preview) {
      return;
    }
    reconstructionRequested = true;
    const baseRequestId = state.current, key = state.sourceId, opts = deps.serializableOptions(state.options);
    // The native command advances its accepted session atomically; do not pre-invalidate it.
    deps.clearParameterSearch();
    state.current++;
    const id = state.current;
    clearTimer();
    state.queued = false;
    state.busy = true;
    deps.resetSourceInspection(false);
    deps.resetStepInspection();
    state.result = null;
    state.errorState = null;
    state.noteKey = null;
    state.recognitionFailed = false;
    state.preview = { ...state.preview, request_id: id };
    state.guidedAttempt = { requestId: id, status: 'pending' };
    state.processing = { requestId: id, phase: 'converting', startedAt: deps.now() };
    const accepts = (responseId: number) => !disposed && acceptsGuidedResult(id, key, state.current, state.sourceId, responseId);
    try {
      const converted = await bridge.invoke<Result>(command, { requestId: id, sourceId: key, baseRequestId, ...payload });
      if (!accepts(converted.request_id)) {
        return;
      }
      state.result = converted;
      state.guidanceHints = converted.guidance_hints;
      state.guidanceHistory = converted.guidance_history;
      state.guidedAttempt = state.guidanceHints.length ? { requestId: id, status: 'success' } : null;
      state.selectedId = '';
      if (state.preview) {
        void deps.prepareSelection(id, key, opts, state.preview, state.sourceRevision);
      }
      deps.onResultReady();
    }
    catch (value) {
      if (accepts(id)) {
        state.errorState = { kind: 'native', value };
        state.guidedAttempt = { requestId: id, status: 'refused' };
        if (state.preview) {
          void deps.prepareSelection(id, key, opts, state.preview, state.sourceRevision);
        }
      }
    }
    finally {
      if (state.processing?.requestId === id) {
        state.processing = null;
      }
      state.busy = false;
      if (state.queued && !disposed) {
        void run();
      }
    }
  }

  function canExport() {
    return !disposed && !!deps.native() && !state.busy
      && !!state.result && state.result.request_id === state.current
      && !!state.result.bodies.length;
  }

  async function exportStep() {
    if (!canExport()) {
      return;
    }
    const id = state.current, bridge = deps.native()!;
    try {
      const path = await bridge.invoke<string | null>('export_step', { requestId: id, suggestedName: state.name.replace(/\.(stl|3mf)$/i, '') + '.step' });
      if (!disposed && id === state.current) {
        state.noteKey = path ? 'workspace.saved' : 'workspace.cancelled';
      }
    }
    catch (value) {
      if (!disposed && id === state.current) {
        state.errorState = { kind: 'native', value };
      }
    }
  }

  function dispose() {
    if (state.largeTriangles && state.busy && state.sourceId !== null) {
      void deps.native()?.invoke<void>('cancel_large_conversion', { requestId: state.current, sourceId: state.sourceId }).catch(() => { /* Supervisor also checks native invalidation. */ });
    }
    disposed = true;
    clearTimer();
    state.current++;
    state.fileToken++;
    state.queued = false;
    state.processing = null;
  }
  return { invalidate, run, load, setSource, reconstruct, canCancelAuto, cancelAuto, cancelLarge, canReconstruct, mutateGuidance, canExport, exportStep, dispose };
}
