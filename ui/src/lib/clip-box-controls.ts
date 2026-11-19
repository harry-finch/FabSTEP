import { Box3, Box3Helper, Group, LineBasicMaterial, Mesh, MeshBasicMaterial, OctahedronGeometry, Raycaster, Vector2, Vector3, type PerspectiveCamera } from 'three';
import { axes, type Axis, type Limit } from './clip-box.ts';

type Drag = {
  id: number; axis: Axis; side: Limit; value: number; initial: Box3;
  x: number; y: number; unitsPerPixel: Vector2;
};

/** The handle owns a pointer from press to release, including cancellation. */
export class ClipBoxControls {
  readonly group = new Group();
  private frame = new Box3Helper(new Box3());
  private frameMaterial = this.frame.material as LineBasicMaterial;
  private geometry = new OctahedronGeometry(1);
  private material = new MeshBasicMaterial({ depthTest: false, depthWrite: false });
  private handles: Mesh[] = [];
  private raycaster = new Raycaster();
  private drag: Drag | null = null;
  private readonly listeners = new AbortController();
  private camera: PerspectiveCamera;
  private canvas: HTMLCanvasElement;
  private options: {
    change: (axis: Axis, side: Limit, value: number) => void;
    restore: (box: Box3) => void;
    orbit: (enabled: boolean) => void;
    available: () => boolean;
  };

  constructor(camera: PerspectiveCamera, canvas: HTMLCanvasElement, options: ClipBoxControls['options']) {
    this.camera = camera;
    this.canvas = canvas;
    this.options = options;
    this.group.visible = false;
    this.frame.renderOrder = 4;
    this.frameMaterial.depthTest = false;
    this.frameMaterial.depthWrite = false;
    this.group.add(this.frame);
    for (const axis of axes) {
      for (const side of ['min', 'max'] as const) {
        const mesh = new Mesh(this.geometry, this.material);
        mesh.userData = { axis, side };
        mesh.renderOrder = 5;
        this.group.add(mesh);
        this.handles.push(mesh);
      }
    }
    const capture = { capture: true, signal: this.listeners.signal };
    canvas.addEventListener('pointerdown', this.down, capture);
    canvas.addEventListener('pointermove', this.move, capture);
    canvas.addEventListener('pointerup', this.up, capture);
    canvas.addEventListener('pointercancel', this.cancelPointer, capture);
    canvas.addEventListener('lostpointercapture', this.cancelPointer, capture);
    window.addEventListener('blur', this.cancel, capture);
    window.addEventListener('keydown', this.key, capture);
  }

  show(box: Box3 | null, handles: boolean) {
    if (!box || !handles) this.cancel();
    this.group.visible = box !== null;
    if (!box) return;
    this.frame.box.copy(box);
    for (const handle of this.handles) {
      const { axis, side } = handle.userData as { axis: Axis; side: Limit };
      box.getCenter(handle.position);
      handle.position[axis] = box[side][axis];
      handle.visible = handles;
    }
  }

  recolor(frame: string, handle: string) {
    this.frameMaterial.color.set(frame);
    this.material.color.set(handle);
  }

  /** Keep handles targetable as the camera zooms. Called once per rendered frame. */
  resize() {
    if (!this.group.visible || !this.canvas.clientHeight) return;
    this.camera.updateMatrixWorld();
    const k = 2 * Math.tan(this.camera.fov * Math.PI / 360) / this.canvas.clientHeight;
    const position = new Vector3();
    for (const handle of this.handles) {
      const depth = -position.copy(handle.position).applyMatrix4(this.camera.matrixWorldInverse).z;
      handle.scale.setScalar(Math.max(depth, this.camera.near) * k * 8);
    }
  }

  private down = (event: PointerEvent) => {
    if (this.drag) { event.stopImmediatePropagation(); return; }
    if (!this.group.visible || !this.handles[0].visible || event.button !== 0 || !event.isPrimary || !this.options.available()) return;
    const rect = this.canvas.getBoundingClientRect();
    if (!rect.width || !rect.height) return;
    this.resize();
    this.group.updateMatrixWorld(true);
    const pointer = new Vector2(2 * (event.clientX - rect.left) / rect.width - 1, 1 - 2 * (event.clientY - rect.top) / rect.height);
    this.raycaster.setFromCamera(pointer, this.camera);
    const hit = this.raycaster.intersectObjects(this.handles, false)[0];
    if (!hit) return;
    this.consume(event);
    const { axis, side } = hit.object.userData as { axis: Axis; side: Limit };
    const position = hit.object.position;
    // Project a unit axis at the handle. Use vertical movement for an axis viewed end-on.
    const cameraPoint = position.clone().applyMatrix4(this.camera.matrixWorldInverse);
    const worldPerPixel = Math.abs(cameraPoint.z) * 2 * Math.tan(this.camera.fov * Math.PI / 360) / rect.height;
    const step = worldPerPixel * 10;
    const a = position.clone().project(this.camera);
    const b = position.clone();
    b[axis] += step;
    b.project(this.camera);
    const screen = new Vector2((b.x - a.x) * rect.width / 2, (a.y - b.y) * rect.height / 2);
    const unitsPerPixel = screen.lengthSq() >= 1
      ? screen.multiplyScalar(step / screen.lengthSq())
      : new Vector2(0, -worldPerPixel);
    this.drag = { id: event.pointerId, axis, side, value: position[axis], initial: this.frame.box.clone(), x: event.clientX, y: event.clientY, unitsPerPixel };
    this.canvas.setPointerCapture(event.pointerId);
    this.options.orbit(false);
  };

  private consume(event: PointerEvent | KeyboardEvent) { event.preventDefault(); event.stopImmediatePropagation(); }

  private update(event: PointerEvent) {
    const drag = this.drag!;
    const delta = (event.clientX - drag.x) * drag.unitsPerPixel.x + (event.clientY - drag.y) * drag.unitsPerPixel.y;
    this.options.change(drag.axis, drag.side, drag.value + delta);
  }

  private move = (event: PointerEvent) => {
    if (!this.drag) return;
    this.consume(event);
    if (this.drag.id !== event.pointerId) return;
    if (!(event.buttons & 1)) { this.cancel(); return; }
    this.update(event);
  };

  private up = (event: PointerEvent) => {
    if (!this.drag) return;
    this.consume(event);
    if (this.drag.id === event.pointerId) { this.update(event); this.finish(false); }
  };

  private cancelPointer = (event: PointerEvent) => {
    if (this.drag?.id === event.pointerId) { this.consume(event); this.cancel(); }
  };

  private key = (event: KeyboardEvent) => {
    if (this.drag && event.key === 'Escape') { this.consume(event); this.cancel(); }
  };

  private finish(restore: boolean) {
    const drag = this.drag;
    if (!drag) return;
    this.drag = null;
    if (restore) this.options.restore(drag.initial);
    if (this.canvas.hasPointerCapture(drag.id)) this.canvas.releasePointerCapture(drag.id);
    this.options.orbit(true);
  }

  cancel = () => this.finish(true);

  dispose() {
    this.listeners.abort();
    this.finish(false);
    this.group.removeFromParent();
    this.frame.geometry.dispose();
    this.frameMaterial.dispose();
    this.geometry.dispose();
    this.material.dispose();
  }
}
