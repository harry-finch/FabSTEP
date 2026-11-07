import { test } from 'node:test';
import { strict as assert } from 'node:assert';
import { bodyLabel, commitSnapshot, errorContext, validPreferences } from '../src/lib/editing.ts';
import type { Options } from '../src/lib/types.ts';

const original: Options = { input_unit:'mm', reconstruction:'surfaces', weld_tolerance_mm:1e-6, plane_tolerance_mm:1e-6, cylinder_fit_tolerance_mm:1e-5, max_deviation_mm:0.05 };

test('a multi-keystroke edit commits one recoverable previous snapshot', () => {
  const intermediate = { ...original, max_deviation_mm:0.0 };
  const finished = { ...intermediate, max_deviation_mm:0.001 };
  assert.equal(commitSnapshot(intermediate, original), null);
  const undo = commitSnapshot(finished, original);
  assert.deepEqual(undo, original);
  assert.notEqual(undo, original);
  assert.equal(commitSnapshot(finished, finished), null);
});

test('invalid inactive settings are never accepted for persistence', () => {
  for (const n of [0, -1, NaN, Infinity]) {
    const draft = { ...original, reconstruction:'planar' as const, cylinder_fit_tolerance_mm:n };
    assert.equal(validPreferences(draft), false);
    assert.equal(commitSnapshot(draft, original), null);
  }
});

test('only generated STL body identifiers receive a translated label', () => {
  assert.equal(bodyLabel({ id:'stl/component/0', name:'Corps STL' }, 'STL body'), 'STL body');
  assert.equal(bodyLabel({ id:'3mf/object/1', name:'Corps STL' }, 'STL body'), 'Corps STL');
  assert.equal(bodyLabel({ id:'3mf/object/2', name:'Part [special]' }, 'STL body'), 'Part [special]');
});


test('structured error context localizes only generated names and preserves detail', () => {
  const stl = errorContext({ code:'recognition.unsupported', message:'Corps STL [stl/component/0] : détail', body:{ id:'stl/component/0', name:'Corps STL', detail:'détail' } }, 'STL body');
  assert.deepEqual(stl, { prefix:'STL body [stl/component/0] : ', detail:'détail' });
  const user = errorContext({ message:'unused', body:{ id:'3mf/object/7', name:'Corps STL [custom]', detail:'detail' } }, 'STL body');
  assert.deepEqual(user, { prefix:'Corps STL [custom] [3mf/object/7] : ', detail:'detail' });
  assert.deepEqual(errorContext({ message:'custom [name] : intact' }, 'STL body'), { prefix:'', detail:'custom [name] : intact' });
});

test('generated 3MF labels use provenance, never the spelling of user names', () => {
  const translate = (object: number, instance: number) => `Object ${object} · ${instance}`;
  const generated = { id:'3mf/build/0', name:'Objet 1 · 1', provenance:{ generated_name:true, object_id:1, build_item:0 } };
  assert.equal(bodyLabel(generated, 'STL body', translate), 'Object 1 · 1');
  assert.equal(bodyLabel({ ...generated, provenance:{ ...generated.provenance, generated_name:false } }, 'STL body', translate), 'Objet 1 · 1');
  assert.equal(bodyLabel({ ...generated, provenance:{ generated_name:true } }, 'STL body', translate), 'Objet 1 · 1');
  const err = { message:'legacy', body:{ id:'3mf/build/0/component/1', name:generated.name, detail:'detail' } };
  assert.deepEqual(errorContext(err, 'STL body', id => id.startsWith(`${generated.id}/component/`) ? bodyLabel(generated, 'STL body', translate) : undefined), { prefix:'Object 1 · 1 [3mf/build/0/component/1] : ', detail:'detail' });
});
