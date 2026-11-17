import { test } from 'node:test';
import { strict as assert } from 'node:assert';
import { isGuidedRefusal } from '../src/lib/guided.ts';
import { createWorkflow, type WorkflowState } from '../src/lib/workflow.ts';
import type { SourceBridge } from '../src/lib/source.ts';
import type { BodyResult, DocumentPreview, GuidedReport, Options, Result } from '../src/lib/types.ts';
const initial: Options = {
  input_unit: 'mm', reconstruction: 'planar', weld_tolerance_mm: 1e-6, plane_tolerance_mm: 1e-6, cylinder_fit_tolerance_mm: 1e-5, max_deviation_mm: 0.05
};
function deferred<T>() {
  let resolve!: (value: T) => void, reject!: (value: unknown) => void;
  const promise = new Promise<T>((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}
const tick = async () => {
  for (let i = 0; i < 12; i++)
    await Promise.resolve();
};
function preview(id: number): DocumentPreview {
  return {
    request_id: id, source: { format: 'stl', unit: 'mm', unit_origin: 'selected' }, bodies: [], diagnostics: []
  };
}
function result(id: number, hints: GuidedReport[] = []): Result {
  const cad: BodyResult['cad'] = {
    solids: 1, faces: 6, planar_faces: 6, cylindrical_faces: 0, conical_faces: 0, spherical_faces: 0, toroidal_faces: 0, volume_mm3: 1, bounds_mm: [[0, 0, 0], [1, 1, 1]], kernel: 'test'
  };
  const body: BodyResult = {
    id: 'A', name: 'A', provenance: {}, analysis: { mesh: { vertices: [], triangles: [] }, model: { vertices: [], faces: [] }, report: {
        input_triangles: 12, vertices: 8, planar_faces: 6, welded_vertices: 0, volume_mm3: 1, bounds_mm: cad.bounds_mm, diagnostics: []
      } }, cad, recognition: null, rounded_recognition: null, revolved_recognition: null
  };
  return {
    request_id: id, settings: { ...initial }, source: preview(id).source, guidance_hints: hints, bodies: [body], cad, diagnostics: []
  };
}
const hint: GuidedReport = {
  hint_id: 1, family: 'cylinder', selected_face_ids: [2, 3], preview_body_id: 'A', analysis_body_id: 'A/component/0', selected_source_triangle_ids: [0], reconstructed_source_triangle_ids: [0, 1]
};
function fixture() {
  const state: WorkflowState = {
    largeTriangles: null, largeAccepted: false, cancelling: false, options: { ...initial }, bytes: null, name: 'part.stl', format: 'stl', current: 1, fileToken: 0, sourceId: null, uploadedSourceId: null, sourceRevision: 0, result: null, preview: null, selectedId: '', errorState: null, noteKey: null, recognitionFailed: false, busy: false, queued: false, processing: null, topologyState: 'unavailable', guidanceHints: [], guidedAttempt: null, fitSignal: 0
  };
  const calls: {
    command: string;
    args: Record<string, unknown> | Uint8Array;
    pending: ReturnType<typeof deferred<unknown>>;
  }[] = [];
  const bridge: SourceBridge = { invoke: async <T>(command: string, args: Record<string, unknown> | Uint8Array) => {
      const pending = deferred<unknown>();
      calls.push({ command, args, pending });
      if (command === 'invalidate_result') {
        pending.resolve(undefined);
      }
      return await pending.promise as T;
    } };
  let native: SourceBridge | undefined = bridge;
  const timers = new Map<number, () => void>();
  let timerId = 0;
  const hooks: string[] = [];
  const workflow = createWorkflow(state, {
    native: () => native, serializableOptions: options => ({ ...options }), now: () => 42,
    clearParameterSearch: () => {
      hooks.push('clearSearch');
    },
    resetSourceInspection: newGeometry => {
      hooks.push(`source:${newGeometry}`);
      if (newGeometry) {
        state.sourceRevision++;
      }
    },
    resetStepInspection: () => {
      hooks.push('step');
    },
    prepareSelection: (id, key, _opts, _doc, revision) => {
      hooks.push(`prepare:${id}:${key}:${revision}`);
    },
    interpretSourceZone: () => {
      hooks.push('interpret');
    },
    onResultReady: () => {
      assert.equal(state.result?.request_id, state.current);
      hooks.push(`ready:${state.current}`);
    },
    schedule: (callback, delay) => {
      assert.equal(delay, 250);
      const id = ++timerId;
      timers.set(id, callback);
      return id;
    },
    clearSchedule: id => {
      timers.delete(id as number);
    }
  });
  const latest = (command: string) => {
    const call = calls.findLast(call => call.command === command);
    assert.ok(call, `Expected ${command}`);
    return call;
  };
  const count = (command: string) => calls.filter(call => call.command === command).length;
  const flush = () => {
    const callbacks = [...timers.values()];
    timers.clear();
    for (const callback of callbacks)
      callback();
  };
  const start = () => {
    workflow.setSource(new Uint8Array([1, 2, 3]));
    void workflow.reconstruct();
    return state.current;
  };
  const uploaded = async () => {
    latest('upload_source').pending.resolve(undefined);
    await tick();
  };
  const previewed = async (id = state.current) => {
    latest('preview_document').pending.resolve(preview(id));
    await tick();
  };
  const converted = async (id = state.current) => {
    latest('convert_document').pending.resolve(result(id));
    await tick();
  };
  const ready = () => {
    state.sourceId = 7;
    state.uploadedSourceId = 7;
    state.bytes = new Uint8Array([1]);
    state.preview = preview(state.current);
    state.result = result(state.current);
  };
  return {
    state, workflow, calls, hooks, latest, count, flush, start, uploaded, previewed, converted, ready, timers, offline: () => {
      native = undefined;
    }
  };
}
test('opening a file uploads and prepares its source preview without reconstructing STEP', async () => {
  const f = fixture();
  await f.workflow.load({ name: 'opened.stl', size: 3, arrayBuffer: async () => new Uint8Array([1, 2, 3]).buffer });
  f.flush();
  await f.uploaded();
  const id = f.state.current;
  await f.previewed();
  assert.equal(f.state.preview?.request_id, id);
  assert.equal(f.state.processing, null);
  assert.equal(f.state.busy, false);
  assert.equal(f.count('convert_document'), 0);
  assert.equal(f.state.result, null);
  assert.equal(f.workflow.canExport(), false);
  assert.equal(f.workflow.canReconstruct(), true);
  assert.equal(f.hooks.some(hook => hook.startsWith('ready:')), false);
});
test('one explicit reconstruction click publishes once and rejects a duplicate while active', async () => {
  const f = fixture();
  f.workflow.setSource(new Uint8Array([1]));
  f.flush();
  await f.uploaded();
  await f.previewed();
  const doc = f.state.preview;
  const operation = f.workflow.reconstruct();
  const id = f.state.current;
  assert.equal(f.state.preview, doc);
  assert.equal(f.workflow.canReconstruct(), false);
  await f.workflow.reconstruct();
  assert.equal(f.state.current, id);
  assert.equal(f.count('preview_document'), 2);
  await f.previewed();
  assert.equal(f.count('convert_document'), 1);
  assert.equal(f.workflow.canExport(), false);
  await f.converted();
  await operation;
  assert.equal(f.workflow.canExport(), true);
  assert.deepEqual(f.hooks.filter(hook => hook.startsWith('ready:')), [`ready:${id}`]);
});
test('options before the first click replace stale preview work without starting conversion', async () => {
  const f = fixture();
  f.workflow.setSource(new Uint8Array([1]));
  f.flush();
  await f.uploaded();
  const oldId = f.state.current, oldPreview = f.latest('preview_document');
  f.state.options = { ...initial, input_unit: 'inch', max_deviation_mm: 0.1 };
  f.workflow.invalidate(true);
  f.flush();
  assert.equal(f.state.queued, true);
  oldPreview.pending.resolve(preview(oldId));
  await tick();
  assert.equal(f.state.preview, null);
  const current = f.latest('preview_document');
  assert.equal((current.args as { options: Options }).options.input_unit, 'inch');
  await f.previewed();
  assert.equal(f.count('convert_document'), 0);
  assert.equal(f.state.preview?.request_id, f.state.current);
  assert.equal(f.workflow.canReconstruct(), true);
  assert.equal(f.state.busy, false);
});
test('opening a replacement after an accepted STEP resets the reconstruction request', async () => {
  const f = fixture();
  f.start();
  await f.uploaded();
  await f.previewed();
  await f.converted();
  assert.equal(f.workflow.canExport(), true);
  await f.workflow.load({ name: 'replacement.3mf', size: 1, arrayBuffer: async () => new Uint8Array([9]).buffer });
  assert.equal(f.state.result, null);
  assert.equal(f.workflow.canExport(), false);
  f.flush();
  await f.uploaded();
  await f.previewed();
  assert.equal(f.count('convert_document'), 1);
  f.state.options = { ...initial, reconstruction: 'auto' };
  f.workflow.invalidate(true);
  f.flush();
  await f.previewed();
  assert.equal(f.count('convert_document'), 1);
  assert.equal(f.workflow.canReconstruct(), true);
  assert.equal(f.state.result, null);
});
for (const failure of [false, true]) {
  test(`replacement preview rejects a late conversion ${failure ? 'failure' : 'success'} and requires its own click`, async () => {
    const f = fixture(), oldId = f.start();
    await f.uploaded();
    await f.previewed();
    const oldConversion = f.latest('convert_document');
    f.workflow.setSource(new Uint8Array([9]));
    f.flush();
    if (failure) oldConversion.pending.reject({ code: 'cad.invalid_solid' });
    else oldConversion.pending.resolve(result(oldId));
    await tick();
    assert.equal(f.state.result, null);
    assert.equal(f.state.errorState, null);
    assert.equal(f.hooks.some(hook => hook.startsWith('ready:')), false);
    await f.uploaded();
    await f.previewed();
    assert.equal(f.count('convert_document'), 1);
    assert.equal(f.workflow.canExport(), false);
    assert.equal(f.workflow.canReconstruct(), true);
    const operation = f.workflow.reconstruct();
    await f.previewed();
    assert.equal(f.count('convert_document'), 2);
    await f.converted();
    await operation;
    assert.equal(f.state.result?.request_id, f.state.current);
  });
}
test('an explicit retry after conversion refusal clears the error and revalidates before export', async () => {
  const f = fixture();
  f.start();
  await f.uploaded();
  await f.previewed();
  f.latest('convert_document').pending.reject({ code: 'recognition.unsupported' });
  await tick();
  assert.ok(f.state.errorState);
  assert.equal(f.workflow.canExport(), false);
  assert.equal(f.workflow.canReconstruct(), true);
  const operation = f.workflow.reconstruct();
  assert.equal(f.state.errorState, null);
  assert.ok(f.state.preview);
  await f.previewed();
  await f.converted();
  await operation;
  assert.equal(f.count('convert_document'), 2);
  assert.equal(f.workflow.canExport(), true);
});
test('preparing a faceted correction resets the old STEP and selection before recalculation', async () => {
  const f = fixture();
  f.ready();
  f.state.options.reconstruction = 'surfaces';
  f.state.result!.settings.reconstruction = 'surfaces';
  const old = f.state.current;
  assert.equal(f.workflow.canExport(),true);
  // The correction button uses the ordinary options-change invalidation path.
  f.state.options = {...f.state.options,reconstruction:'planar'};
  f.workflow.invalidate(true);
  assert.equal(f.workflow.canExport(),false);
  assert.equal(f.state.result,null);
  assert.equal(f.state.current,old+1);
  assert.ok(f.hooks.includes('step'));
  assert.deepEqual(f.state.guidanceHints,[]);
  await f.workflow.exportStep();
  assert.equal(f.count('export_step'),0);
  f.flush();
  await f.previewed();
  const conversion = f.latest('convert_document');
  assert.equal((conversion.args as {options:Options}).options.reconstruction,'planar');
  assert.equal(f.workflow.canExport(),false);
  await f.converted();
  assert.equal(f.workflow.canExport(),true);
});
test('file import validates before reading and only the latest file read can publish', async () => {
  const f = fixture(), first = deferred<ArrayBuffer>(), second = deferred<ArrayBuffer>();
  const a = f.workflow.load({ name: 'old.stl', size: 1, arrayBuffer: () => first.promise });
  const b = f.workflow.load({ name: 'new.3mf', size: 1, arrayBuffer: () => second.promise });
  second.resolve(new Uint8Array([2]).buffer);
  await b;
  const id = f.state.current;
  first.resolve(new Uint8Array([1]).buffer);
  await a;
  assert.deepEqual([...f.state.bytes!], [2]);
  assert.equal(f.state.name, 'new.3mf');
  assert.equal(f.state.format, '3mf');
  assert.equal(f.state.current, id);
  assert.equal(f.state.fitSignal, 1);
  let read = false;
  for (const [name, size, key] of [['bad.txt', 1, 'workspace.formatHint'], ['large.stl', 64 * 1024 * 1024 + 1, 'workspace.tooLarge']] as const) {
    await f.workflow.load({ name, size, arrayBuffer: async () => {
        read = true;
        return new ArrayBuffer(0);
      } });
    assert.deepEqual(f.state.errorState, { kind: 'key', key });
    assert.equal(f.state.result, null);
    assert.equal(f.state.bytes, null);
  }
  assert.equal(read, false);
  assert.equal(f.count('upload_source'), 0);
});
test('file read errors are visible only for the current file', async () => {
  const f = fixture(), old = deferred<ArrayBuffer>();
  const first = f.workflow.load({ name: 'old.stl', size: 1, arrayBuffer: () => old.promise });
  await f.workflow.load({ name: 'new.stl', size: 1, arrayBuffer: async () => {
      throw Error('read');
    } });
  assert.deepEqual(f.state.errorState, { kind: 'key', key: 'workspace.errorFile' });
  f.state.errorState = null;
  old.reject(Error('late read'));
  await first;
  assert.equal(f.state.errorState, null);
});
test('explicit reconstruction uploads, previews and converts before enabling export', async () => {
  const f = fixture(), id = f.start();
  assert.deepEqual(f.hooks, ['clearSearch', 'source:true', 'step', 'clearSearch', 'source:false', 'step']);
  assert.equal(f.state.processing?.phase, 'preview');
  assert.equal(f.workflow.canExport(), false);
  assert.deepEqual([...f.latest('upload_source').args as Uint8Array], [1, 2, 3]);
  await f.uploaded();
  assert.equal(f.state.uploadedSourceId, f.state.sourceId);
  assert.deepEqual(f.latest('preview_document').args, { requestId: id, sourceId: f.state.sourceId, options: initial });
  await f.previewed();
  assert.equal(f.state.processing?.phase, 'converting');
  assert.equal(f.state.preview?.request_id, id);
  assert.equal(f.workflow.canExport(), false);
  await f.converted();
  assert.equal(f.state.processing, null);
  assert.equal(f.state.busy, false);
  assert.equal(f.workflow.canExport(), true);
  assert.ok(f.hooks.includes(`prepare:${id}:${f.state.sourceId}:${f.state.sourceRevision}`));
  assert.deepEqual(f.hooks.slice(-2), ['interpret', `ready:${id}`]);
});
test('current conversion refusal preserves imported preview and recognition diagnostic', async () => {
  const f = fixture();
  f.state.options.reconstruction = 'surfaces';
  f.start();
  await f.uploaded();
  await f.previewed();
  const doc = f.state.preview, error = { code: 'recognition.partial_unproven', message: 'test' };
  f.latest('convert_document').pending.reject(error);
  await tick();
  assert.equal(f.state.preview, doc);
  assert.equal(f.state.result, null);
  assert.deepEqual(f.state.errorState, { kind: 'native', value: error });
  assert.equal(f.state.recognitionFailed, true);
  assert.equal(f.workflow.canExport(), false);
});
for (const phase of ['upload_source', 'preview_document', 'convert_document'] as const) {
  for (const failure of [false, true]) {
    test(`options change rejects late ${phase} ${failure ? 'error' : 'success'} and runs queued request`, async () => {
      const f = fixture(), oldId = f.start();
      if (phase !== 'upload_source') {
        await f.uploaded();
      }
      if (phase === 'convert_document') {
        await f.previewed();
      }
      const old = f.latest(phase), source = f.state.sourceId;
      f.state.options = { ...initial, max_deviation_mm: 0.1 };
      f.workflow.invalidate(true);
      f.flush();
      const id = f.state.current;
      assert.equal(f.state.busy, true);
      assert.equal(f.state.queued, true);
      assert.equal(f.state.processing?.requestId, id);
      if (failure) {
        old.pending.reject(Error('stale'));
      }
      else
        old.pending.resolve(phase === 'preview_document' ? preview(oldId) : phase === 'convert_document' ? result(oldId) : undefined);
      await tick();
      assert.equal(f.state.errorState, null);
      assert.equal(f.state.result, null);
      assert.equal(f.state.current, id);
      assert.equal(f.state.sourceId, source);
      assert.equal(f.state.processing?.requestId, id);
      if (phase === 'upload_source' && failure) {
        await f.uploaded();
      }
      assert.equal((f.latest('preview_document').args as Record<string, unknown>).requestId, id);
      await f.previewed();
      await f.converted();
      assert.equal(f.state.result?.request_id, id);
      assert.equal(f.state.busy, false);
      assert.equal(f.count('upload_source'), phase === 'upload_source' && failure ? 2 : 1);
    });
  }
}
test('a replacement source never inherits upload completion or result from the previous source', async () => {
  const f = fixture();
  f.start();
  const upload = f.latest('upload_source'), oldSource = f.state.sourceId;
  f.workflow.setSource(new Uint8Array([9]));
  f.flush();
  upload.pending.resolve(undefined);
  await tick();
  assert.notEqual(f.state.sourceId, oldSource);
  assert.equal(f.state.uploadedSourceId, null);
  assert.equal(f.count('upload_source'), 2);
  assert.deepEqual([...f.latest('upload_source').args as Uint8Array], [9]);
  await f.uploaded();
  await f.previewed();
  assert.equal(f.count('convert_document'), 0);
  assert.equal(f.state.result, null);
  void f.workflow.reconstruct();
  await f.previewed();
  await f.converted();
  assert.equal(f.state.result?.request_id, f.state.current);
});
test('invalid options cancel pending work and valid options prepare a preview before the first click', async () => {
  const f = fixture();
  f.workflow.setSource(new Uint8Array([1]));
  f.state.options = { ...initial, weld_tolerance_mm: NaN };
  f.workflow.invalidate(true);
  f.flush();
  assert.equal(f.count('upload_source'), 0);
  assert.equal(f.state.processing, null);
  assert.equal(f.state.topologyState, 'unavailable');
  assert.deepEqual(f.state.errorState, { kind: 'key', key: 'workspace.invalid' });
  f.state.options = { ...initial };
  f.workflow.invalidate(true);
  f.flush();
  await f.uploaded();
  await f.previewed();
  assert.equal(f.count('convert_document'), 0);
  assert.equal(f.workflow.canExport(), false);
  void f.workflow.reconstruct();
  await f.previewed();
  await f.converted();
  assert.equal(f.workflow.canExport(), true);
});
test('stale response request identifiers cannot publish preview or conversion', async () => {
  const f = fixture(), id = f.start();
  await f.uploaded();
  await f.previewed(id - 1);
  assert.equal(f.state.preview, null);
  assert.equal(f.count('convert_document'), 0);
  f.workflow.invalidate();
  f.flush();
  await f.previewed();
  await f.converted(f.state.current - 1);
  assert.equal(f.state.result, null);
  assert.equal(f.workflow.canExport(), false);
});
test('guided commands preserve their native base, accepted hints and source preview across refusal and recovery', async () => {
  const f = fixture();
  f.ready();
  f.state.guidanceHints = [hint];
  const base = f.state.current;
  const mutation = f.workflow.mutateGuidance('guided_convert', { faceIds: [4, 5] });
  assert.equal(f.count('invalidate_result'), 0);
  assert.deepEqual(f.hooks, ['clearSearch', 'source:false', 'step']);
  assert.equal(f.state.result, null);
  assert.deepEqual(f.state.guidanceHints, [hint]);
  assert.equal(f.state.preview?.request_id, base + 1);
  assert.deepEqual(f.latest('guided_convert').args, {
    requestId: base + 1, sourceId: 7, baseRequestId: base, faceIds: [4, 5]
  });
  f.latest('guided_convert').pending.reject({ code: 'guidance.cylinder_unproven' });
  await mutation;
  assert.equal(f.state.guidedAttempt?.status, 'refused');
  assert.deepEqual(f.state.guidanceHints, [hint]);
  assert.equal(f.workflow.canExport(), false);
  const recover = f.workflow.mutateGuidance('recompute_guidance');
  assert.equal((f.latest('recompute_guidance').args as Record<string, unknown>).baseRequestId, base + 1);
  f.latest('recompute_guidance').pending.resolve(result(f.state.current, [hint]));
  await recover;
  assert.equal(f.state.guidedAttempt?.status, 'success');
  assert.equal(f.workflow.canExport(), true);
  const remove = f.workflow.mutateGuidance('remove_guidance', { hintId: 1 });
  assert.deepEqual(f.state.guidanceHints, [hint]);
  assert.equal((f.latest('remove_guidance').args as Record<string, unknown>).hintId, 1);
  f.latest('remove_guidance').pending.resolve(result(f.state.current));
  await remove;
  assert.deepEqual(f.state.guidanceHints, []);
  assert.equal(f.state.guidedAttempt, null);
  assert.equal(f.count('invalidate_result'), 0);
});
for (const failure of [false, true]) {
  test(`options invalidation rejects late guided ${failure ? 'refusal' : 'success'} and resumes conversion`, async () => {
    const f = fixture();
    f.ready();
    f.state.guidanceHints = [hint];
    const mutation = f.workflow.mutateGuidance('remove_guidance', { hintId: 1 });
    const id = f.state.current;
    f.state.options = { ...initial, plane_tolerance_mm: 1e-7 };
    f.workflow.invalidate(true);
    f.flush();
    if (failure) {
      f.latest('remove_guidance').pending.reject(Error('stale'));
    }
    else
      f.latest('remove_guidance').pending.resolve(result(id, [hint]));
    await mutation;
    await tick();
    assert.equal(f.state.result, null);
    assert.equal(f.state.errorState, null);
    assert.deepEqual(f.state.guidanceHints, []);
    assert.equal(f.state.guidedAttempt, null);
    assert.equal((f.latest('preview_document').args as Record<string, unknown>).requestId, f.state.current);
    await f.previewed();
    await f.converted();
    assert.equal(f.workflow.canExport(), true);
  });
}
test('guided command guards reject missing source, preview, bridge and engine activity', async () => {
  for (const blocked of ['source', 'preview', 'bridge', 'busy', 'processing'] as const) {
    const f = fixture();
    f.ready();
    if (blocked === 'source') {
      f.state.sourceId = null;
    }
    if (blocked === 'preview') {
      f.state.preview = null;
    }
    if (blocked === 'bridge') {
      f.offline();
    }
    if (blocked === 'busy') {
      f.state.busy = true;
    }
    if (blocked === 'processing') {
      f.state.processing = { requestId: 1, phase: 'searching', startedAt: 0 };
    }
    await f.workflow.mutateGuidance('recompute_guidance');
    assert.equal(f.count('recompute_guidance'), 0);
  }
});
test('export enforces current complete result guard and reports save, cancel and error', async () => {
  for (const blocked of ['result', 'stale', 'empty', 'busy', 'bridge'] as const) {
    const f = fixture();
    f.ready();
    if (blocked === 'result') {
      f.state.result = null;
    }
    if (blocked === 'stale') {
      f.state.result!.request_id--;
    }
    if (blocked === 'empty') {
      f.state.result!.bodies = [];
    }
    if (blocked === 'busy') {
      f.state.busy = true;
    }
    if (blocked === 'bridge') {
      f.offline();
    }
    await f.workflow.exportStep();
    assert.equal(f.count('export_step'), 0);
  }
  const f = fixture();
  f.ready();
  for (const path of ['/tmp/part.step', null]) {
    const operation = f.workflow.exportStep();
    assert.deepEqual(f.latest('export_step').args, { requestId: f.state.current, suggestedName: 'part.step' });
    f.latest('export_step').pending.resolve(path);
    await operation;
    assert.equal(f.state.noteKey, path ? 'workspace.saved' : 'workspace.cancelled');
  }
  const operation = f.workflow.exportStep(), error = Error('export');
  f.latest('export_step').pending.reject(error);
  await operation;
  assert.deepEqual(f.state.errorState, { kind: 'native', value: error });
});
for (const failure of [false, true]) {
  test(`late export ${failure ? 'error' : 'success'} cannot alter a replacement request`, async () => {
    const f = fixture();
    f.ready();
    const operation = f.workflow.exportStep();
    f.workflow.invalidate(true);
    if (failure) {
      f.latest('export_step').pending.reject(Error('late'));
    }
    else
      f.latest('export_step').pending.resolve('/tmp/old.step');
    await operation;
    assert.equal(f.state.noteKey, null);
    assert.equal(f.state.errorState, null);
  });
}
test('dispose rejects pending reads, conversion and export publication and cancels timers', async () => {
  const read = fixture(), data = deferred<ArrayBuffer>();
  const loading = read.workflow.load({ name: 'a.stl', size: 1, arrayBuffer: () => data.promise });
  read.workflow.dispose();
  data.resolve(new Uint8Array([1]).buffer);
  await loading;
  assert.equal(read.state.bytes, null);
  assert.equal(read.state.fitSignal, 0);
  const convert = fixture();
  convert.start();
  await convert.uploaded();
  await convert.previewed();
  const id = convert.state.current;
  convert.workflow.dispose();
  await convert.converted(id);
  assert.equal(convert.state.result, null);
  assert.equal(convert.state.processing, null);
  assert.equal(convert.state.busy, false);
  const exp = fixture();
  exp.ready();
  const exporting = exp.workflow.exportStep();
  exp.workflow.dispose();
  exp.latest('export_step').pending.resolve('/tmp/a.step');
  await exporting;
  assert.equal(exp.state.noteKey, null);
  assert.equal(exp.workflow.canExport(), false);
  const scheduled = fixture();
  scheduled.workflow.setSource(new Uint8Array([1]));
  assert.equal(scheduled.timers.size, 1);
  scheduled.workflow.dispose();
  scheduled.flush();
  assert.equal(scheduled.timers.size, 0);
  assert.equal(scheduled.count('upload_source'), 0);
});
for (const phase of ['upload_source', 'preview_document'] as const) {
  test(`current ${phase} failure clears processing and marks topology unavailable`, async () => {
    const f = fixture();
    f.start();
    if (phase === 'preview_document') {
      await f.uploaded();
    }
    const error = { code: 'engine.source_transport', message: 'test' };
    f.latest(phase).pending.reject(error);
    await tick();
    assert.deepEqual(f.state.errorState, { kind: 'native', value: error });
    assert.equal(f.state.preview, null);
    assert.equal(f.state.result, null);
    assert.equal(f.state.topologyState, 'unavailable');
    assert.equal(f.state.processing, null);
    assert.equal(f.state.busy, false);
    assert.equal(f.count('convert_document'), 0);
  });
}

test('STEP display is requested once per published conversion, never for an obsolete result or a refusal', async () => {
  const f = fixture();
  const first = f.start();
  await f.uploaded();
  await f.previewed();
  assert.deepEqual(f.hooks.filter(hook => hook.startsWith('ready:')), []);
  const old = f.latest('convert_document');
  f.workflow.invalidate(true);
  f.flush();
  old.pending.resolve(result(first));
  await tick();
  assert.deepEqual(f.hooks.filter(hook => hook.startsWith('ready:')), []);
  await f.previewed();
  await f.converted();
  const accepted = f.state.current;
  assert.deepEqual(f.hooks.filter(hook => hook.startsWith('ready:')), [`ready:${accepted}`]);

  f.workflow.invalidate(true);
  f.flush();
  await f.previewed();
  f.latest('convert_document').pending.reject({code:'recognition.unsupported'});
  await tick();
  assert.deepEqual(f.hooks.filter(hook => hook.startsWith('ready:')), [`ready:${accepted}`]);

  f.workflow.invalidate(true);
  f.flush();
  await f.previewed();
  await f.converted();
  assert.deepEqual(f.hooks.filter(hook => hook.startsWith('ready:')), [`ready:${accepted}`, `ready:${f.state.current}`]);
});

test('guided add, removal and recomputation request STEP display only after accepted success', async () => {
  const f = fixture();
  f.ready();
  const accepted: string[] = [];
  for (const command of ['guided_convert', 'remove_guidance', 'recompute_guidance'] as const) {
    const operation = f.workflow.mutateGuidance(command);
    assert.deepEqual(f.hooks.filter(hook => hook.startsWith('ready:')), accepted);
    f.latest(command).pending.resolve(result(f.state.current, [hint]));
    await operation;
    accepted.push(`ready:${f.state.current}`);
    assert.deepEqual(f.hooks.filter(hook => hook.startsWith('ready:')), accepted);
  }
  const refused = f.workflow.mutateGuidance('guided_convert');
  f.latest('guided_convert').pending.reject({code:'guidance.cylinder_unproven'});
  await refused;
  assert.deepEqual(f.hooks.filter(hook => hook.startsWith('ready:')), accepted);

  const stale = f.workflow.mutateGuidance('recompute_guidance');
  const response = result(f.state.current, [hint]);
  f.workflow.invalidate(true);
  f.latest('recompute_guidance').pending.resolve(response);
  await stale;
  assert.deepEqual(f.hooks.filter(hook => hook.startsWith('ready:')), accepted);
});

for (const family of ['auto', 'plane', 'cylinder', 'cone', 'torus'] as const) {
  test(`guided ${family} dispatch captures the submitted family and publishes native reports`, async () => {
    const f = fixture();
    f.ready();
    const payload = { faceIds: [4, 5], family };
    const base = f.state.current;
    const operation = f.workflow.mutateGuidance('guided_convert', payload);
    payload.family = family === 'cone' ? 'cylinder' : 'cone';
    assert.deepEqual(f.latest('guided_convert').args, {
      requestId: base + 1, sourceId: 7, baseRequestId: base, faceIds: [4, 5], family
    });
    const nativeHints: GuidedReport[] = [hint, { ...hint, hint_id: 2, family }];
    f.latest('guided_convert').pending.resolve(result(f.state.current, nativeHints));
    await operation;
    assert.deepEqual(f.state.guidanceHints, nativeHints);
    assert.equal(f.workflow.canExport(), true);
  });
}
for (const family of ['auto', 'plane', 'cone', 'torus'] as const) {
  for (const outcome of ['success', 'refusal'] as const) {
    test(`source replacement rejects late ${family} ${outcome} without restoring hints or export`, async () => {
      const f = fixture();
      f.ready();
      f.state.guidanceHints = [hint];
      const operation = f.workflow.mutateGuidance('guided_convert', {faceIds:[4, 5], family});
      const id = f.state.current;
      f.workflow.setSource(new Uint8Array([9]));
      if (outcome === 'success') f.latest('guided_convert').pending.resolve(result(id, [{...hint, family}]));
      else f.latest('guided_convert').pending.reject({code:`guidance.${family}_unproven`});
      await operation;
      assert.equal(f.state.result, null);
      assert.equal(f.state.errorState, null);
      assert.deepEqual(f.state.guidanceHints, []);
      assert.equal(f.state.guidedAttempt, null);
      assert.equal(f.workflow.canExport(), false);
      assert.equal(f.hooks.some(hook => hook.startsWith('ready:')), false);
    });
  }
}

for (const command of ['guided_convert', 'remove_guidance', 'recompute_guidance'] as const) {
  test(`failure presentation identifies current ${command} without restoring export`, async () => {
    const f = fixture();
    f.ready();
    const isRefusal = () => isGuidedRefusal(f.state.guidedAttempt, f.state.current, !!f.state.errorState);
    const mutation = f.workflow.mutateGuidance(command);
    assert.equal(isRefusal(), false);
    f.latest(command).pending.reject({ code: 'guidance.family_unproven' });
    await mutation;
    assert.equal(isRefusal(), true);
    assert.equal(f.workflow.canExport(), false);
    const refused = f.state.guidedAttempt;
    f.workflow.invalidate(true);
    assert.equal(isRefusal(), false);
    assert.equal(isGuidedRefusal(refused, f.state.current, true), false);
    f.flush();
    await f.previewed();
    f.latest('convert_document').pending.reject({ code: 'cad.invalid_model' });
    await tick();
    assert.ok(f.state.errorState);
    assert.equal(isRefusal(), false);
  });
}

function denseStl(count = 100_001) {
  const bytes = new Uint8Array(84 + count * 50);
  new DataView(bytes.buffer).setUint32(80, count, true);
  return bytes;
}

test('dense binary STL waits for explicit consent and skips ordinary native preview', async () => {
  const f = fixture();
  f.workflow.setSource(denseStl());
  f.flush(); await tick();
  assert.equal(f.state.largeTriangles, 100_001);
  assert.equal(f.count('upload_source'), 0);
  assert.equal(f.state.busy, false);
  await f.workflow.reconstruct();
  assert.equal(f.count('convert_large_document'), 0);
  const task = f.workflow.reconstruct(true);
  await f.uploaded();
  assert.equal(f.count('preview_document'), 0);
  assert.equal(f.count('convert_document'), 0);
  const call = f.latest('convert_large_document');
  assert.equal((call.args as Record<string, unknown>).confirmed, true);
  call.pending.resolve(result(f.state.current));
  await task;
  assert.ok(f.workflow.canExport());
  assert.equal(f.state.preview, null);
  assert.equal(f.hooks.filter(hook => hook.startsWith('prepare:')).length, 0);
});

test('dense cancellation waits for supervisor and rejects even a late successful result', async () => {
  const f = fixture(); f.workflow.setSource(denseStl());
  const task = f.workflow.reconstruct(true); await f.uploaded();
  const id = f.state.current;
  const cancel = f.workflow.cancelLarge();
  assert.equal(f.state.cancelling, true);
  assert.equal(f.workflow.canExport(), false);
  f.latest('convert_large_document').pending.resolve(result(id));
  await task;
  assert.equal(f.state.result, null);
  assert.equal(f.state.cancelling, true);
  assert.equal(f.workflow.canReconstruct(), false);
  f.latest('cancel_large_conversion').pending.resolve(undefined);
  await cancel;
  assert.equal(f.state.cancelling, false);
  assert.equal(f.state.noteKey, 'workspace.cancelled');
  assert.equal(f.workflow.canReconstruct(), true);
  assert.ok(f.state.bytes);
});

test('dense consent resets on a new file and cancellation during upload prevents spawning', async () => {
  const f = fixture(); f.workflow.setSource(denseStl());
  const task = f.workflow.reconstruct(true);
  const cancel = f.workflow.cancelLarge();
  await f.uploaded(); await task;
  assert.equal(f.count('convert_large_document'), 0);
  f.latest('cancel_large_conversion').pending.resolve(undefined); await cancel;
  f.workflow.setSource(denseStl(100_002)); f.flush(); await tick();
  assert.equal(f.state.largeAccepted, false);
  assert.equal(f.count('upload_source'), 1);
  assert.equal(f.count('convert_large_document'), 0);
});
