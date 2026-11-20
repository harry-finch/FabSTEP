// Run verifyNavigation(cubeStlText) on the Vite dev server with fixtures/cube.stl.
import { mount, unmount } from 'svelte';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
import Viewport from '../src/components/Viewport.svelte';
import { prepareStepDisplay } from '../src/lib/step-display.ts';

export async function verifyNavigation(cube) {
  const vertices = Array.from(cube.matchAll(/vertex\s+(\S+)\s+(\S+)\s+(\S+)/g), m => m.slice(1).map(Number));
  const triangles = Array.from({length: vertices.length / 3}, (_, i) => [3*i, 3*i+1, 3*i+2]);
  const faces = Array.from({length: 6}, (_, id) => ({
    id, solid_id: 0, family: 'plane', triangle_start: 2*id, triangle_count: 2, adjacent_face_ids: [],
  }));
  const step = {mesh: {vertices, triangles}, faces};
  const props = {
    bytes: new TextEncoder().encode(cube), label: 'Navigation', results: [], sourceRevision: 1,
    unit: 'mm', format: 'stl', fitSignal: 0, stepTriangles: [], onmiss() {}, onfacepaint() {},
  };
  async function exercise(isStep, selectionMode) {
    const target = document.createElement('div');
    target.style.cssText = 'position:fixed;inset:0';
    document.body.appendChild(target);
    let orbit;
    const update = OrbitControls.prototype.update;
    OrbitControls.prototype.update = function(...args) { orbit = this; return update.apply(this, args); };
    let mounted;
    try {
      mounted = mount(Viewport, {target, props: {
        ...props, selectionMode, stepPreview: isStep ? step : null, stepDisplay: isStep ? prepareStepDisplay(step) : null,
      }});
      await new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)));
      const canvas = target.querySelector('canvas');
      // Synthetic events have no OS pointer ID. Only emulate DOM capture, not the controls.
      canvas.setPointerCapture = canvas.releasePointerCapture = () => {};
      const state = () => [...orbit.object.position.toArray(), ...orbit.target.toArray()];
      const moved = (a, b) => a.some((x, i) => Math.abs(x - b[i]) > 1e-9);
      function drag(button, modifier = {}) {
        const before = state();
        const box = canvas.getBoundingClientRect();
        const init = {bubbles:true, pointerType:'mouse', pointerId:1, isPrimary:true, button, ...modifier};
        for (const [type, dx, dy] of [['pointerdown',0,0], ['pointermove',38,27], ['pointerup',38,27]]) {
          canvas.dispatchEvent(new PointerEvent(type, {...init, buttons:type==='pointerup'?0:[1,4,2][button],
            clientX:box.left+box.width/2+dx, clientY:box.top+box.height/2+dy}));
        }
        const after = state();
        return {turned:moved(before.slice(0,3),after.slice(0,3)), panned:moved(before.slice(3),after.slice(3))};
      }
      const right = drag(2), middle = drag(1), shiftRight = drag(2, {shiftKey:true}), left = drag(0), alt = drag(0, {altKey:true});
      const before = orbit.object.position.distanceTo(orbit.target);
      canvas.dispatchEvent(new WheelEvent('wheel', {bubbles:true, cancelable:true, deltaY:100}));
      const zoomed = Math.abs(orbit.object.position.distanceTo(orbit.target) - before) > 1e-9;
      if (!right.turned || right.panned || !middle.panned || !shiftRight.panned || !alt.turned || alt.panned
        || !zoomed || left.panned || left.turned === selectionMode) {
        throw Error(JSON.stringify({isStep, selectionMode, right, middle, shiftRight, left, alt, zoomed}));
      }
      return state();
    } finally {
      if (mounted) await unmount(mounted);
      OrbitControls.prototype.update = update;
      target.remove();
    }
  }
  for (const selecting of [false, true]) {
    const source = await exercise(false, selecting), result = await exercise(true, selecting);
    if (source.some((x,i) => Math.abs(x-result[i]) > 1e-9)) throw Error('Different camera after identical gestures');
  }
  return 'Source and STEP share orbit, pan and zoom with selection on and off';
}
