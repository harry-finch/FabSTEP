// On the Vite dev server, call verifyClipBox(cubeStlText) with fixtures/cube.stl.
export async function verifyClipBox(cube) {
  const second = cube.replace(/vertex\s+(\S+)/g, (_, x) => 'vertex ' + (Number(x) + 2));
  const solids = cube.replace(/endsolid[^\n]*\s*$/, '') + second.replace(/^\s*solid[^\n]*\n/, '');
  const transfer = new DataTransfer();
  transfer.items.add(new File([solids], 'two-cubes.stl'));
  const input = await until(() => document.querySelector('input[type=file]'));
  const source = await compareFrames(document, () => {
    input.files = transfer.files;
    input.dispatchEvent(new Event('change', {bubbles:true}));
  });

  // Feed the same triangles to the coloured STEP renderer, grouped by cube face.
  const { mount, unmount } = await import('svelte');
  const { default: Viewport } = await import('../src/components/Viewport.svelte');
  const { prepareStepDisplay } = await import('../src/lib/step-display.ts');
  const vertices = Array.from(solids.matchAll(/vertex\s+(\S+)\s+(\S+)\s+(\S+)/g), m => m.slice(1).map(Number));
  const triangles = Array.from({length:24}, (_, i) => [3*i, 3*i+1, 3*i+2]);
  const faces = Array.from({length:12}, (_, id) => ({
    id, solid_id:Math.floor(id/6), family:'plane', triangle_start:id*2, triangle_count:2, adjacent_face_ids:[],
  }));
  const stepPreview = { mesh:{vertices, triangles}, faces };
  const target = document.createElement('div');
  target.style.cssText = 'position:fixed;inset:0';
  document.body.appendChild(target);
  let component;
  try {
    const step = await compareFrames(target, () => {
      component = mount(Viewport, {target, props:{
        label:'STEP', stepPreview, stepDisplay:prepareStepDisplay(stepPreview), stepTriangles:[],
        results:[], format:'stl', unit:'mm', sourceRevision:1, showEdges:true,
      }});
    });
    return {source, step};
  } finally {
    if (component) await unmount(component);
    target.remove();
  }
}

const delay = ms => new Promise(resolve => setTimeout(resolve, ms));
async function until(read) {
  for (let i = 0; i < 120; i++) {
    const value = read();
    if (value) return value;
    await delay(35);
  }
  throw Error('Missing viewport frame or box controls');
}

async function compareFrames(root, load) {
  const prototype = WebGL2RenderingContext.prototype;
  const original = prototype.drawArrays;
  let pixels;
  prototype.drawArrays = function(mode, start, count) {
    original.call(this, mode, start, count);
    // Read the model draw before the box frame and its handles are painted.
    if (mode === this.TRIANGLES && count === 72 && root.contains(this.canvas)) {
      pixels = new Uint8Array(this.drawingBufferWidth * this.drawingBufferHeight * 4);
      this.readPixels(0, 0, this.drawingBufferWidth, this.drawingBufferHeight, this.RGBA, this.UNSIGNED_BYTE, pixels);
    }
  };
  async function frame(change) {
    pixels = null;
    change();
    return until(() => pixels);
  }
  function changed(a, b) {
    let count = 0;
    for (let i = 0; i < a.length; i += 4) {
      if (a[i] !== b[i] || a[i+1] !== b[i+1] || a[i+2] !== b[i+2]) count++;
    }
    return count;
  }
  try {
    await frame(load);
    await delay(150);
    const originalPixels = pixels;
    const toggle = root.querySelector('.clip-tool button[aria-pressed]');
    const full = await frame(() => toggle.click());
    root.querySelector('.clip-tool button[aria-expanded]').click();
    const minX = await until(() => root.querySelector('.clip-tool input'));
    const setMin = value => frame(() => {
      minX.value = value;
      minX.dispatchEvent(new Event('change', {bubbles:true}));
    });
    const isolated = await setMin('1.9');
    const coplanar = await setMin('2');
    const sliced = await setMin('2.5');
    const reset = await frame(() => root.querySelector('.clip-tool fieldset button').click());
    const disabled = await frame(() => toggle.click());
    const result = {
      full: changed(originalPixels, full), coplanar: changed(isolated, coplanar),
      reset: changed(originalPixels, reset), disabled: changed(originalPixels, disabled),
      isolated: changed(full, isolated), sliced: changed(coplanar, sliced),
    };
    if (result.full || result.coplanar || result.reset || result.disabled || !result.isolated || !result.sliced) {
      throw Error(JSON.stringify(result));
    }
    return result;
  } finally {
    prototype.drawArrays = original;
  }
}
