// Open fixtures/cube.stl in the source view of the Vite dev server, then run
// await (await import('/tests/webgl-restore.browser.js')).verifyCubeRestore()
export async function verifyCubeRestore() {
  const canvas = document.querySelector('.viewport canvas');
  const gl = canvas?.getContext('webgl2');
  const extension = gl?.getExtension('WEBGL_lose_context');
  if (!extension || gl.isContextLost()) throw Error('A live WebGL2 viewport is required');

  const listeners = new AbortController();
  const once = { once: true, signal: listeners.signal };
  const draw = gl.drawArrays;
  let restored = false;
  let timer;
  try {
    const painted = new Promise((resolve, reject) => {
      timer = setTimeout(() => reject(Error('No cube draw after context restoration')), 5000);
      canvas.addEventListener('webglcontextlost', () => {
        // Restoration can be requested once the cancelable loss event has finished.
        setTimeout(() => extension.restoreContext(), 100);
      }, once);
      canvas.addEventListener('webglcontextrestored', () => { restored = true; }, once);
      gl.drawArrays = function(mode, first, count) {
        draw.call(this, mode, first, count);
        if (restored && mode === gl.TRIANGLES && count === 36) resolve();
      };
    });
    extension.loseContext();
    await painted;
    if (canvas !== document.querySelector('.viewport canvas') || gl !== canvas.getContext('webgl2')) {
      throw Error('Context restoration replaced the viewport');
    }
  } finally {
    clearTimeout(timer);
    listeners.abort();
    delete gl.drawArrays;
  }
}
