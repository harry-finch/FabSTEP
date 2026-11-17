import { test } from 'node:test';
import { strict as assert } from 'node:assert';
import { ContextReveal } from '../src/lib/context-reveal.ts';

test('context reveal overrides a collapsed section only for a new nonempty selection', () => {
  const context = new ContextReveal();
  let open = false; // Existing persisted preference.
  const sync = (key:string|null|undefined) => { if (context.next(key)) open=true; };
  sync(undefined); assert.equal(open,false);
  sync(null); assert.equal(open,false);
  sync('result-1:0,1'); assert.equal(open,true);
  open=false; // Manual collapse after inspection.
  sync('result-1:0,1'); assert.equal(open,false); // Rerender or inspector remount.
  sync('result-1:0,1,2'); assert.equal(open,true); // Grown selection.
  open=false;
  sync(null); assert.equal(open,false);
  sync('result-1:0,1,2'); assert.equal(open,true); // Clear then select again.
  open=false;
  sync('result-2:0,1,2'); assert.equal(open,true); // Same face IDs in a new result.
});
