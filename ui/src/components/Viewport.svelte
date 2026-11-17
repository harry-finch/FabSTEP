<script lang="ts">
  // Vue 3D à la demande : orbite, zoom, cadrage et contours des faces reconnues.
  import { onMount } from 'svelte';
  import * as THREE from 'three';
  import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
  import { STLLoader } from 'three/examples/jsm/loaders/STLLoader.js';
  import { largeStlTriangles } from '../lib/large-stl';
  import { createStlPreviewWorker } from '../lib/stl-preview-worker';
  import {TransformControls} from 'three/examples/jsm/controls/TransformControls.js';
  import { STEP_PALETTE_SIZE, type StepDisplay } from '../lib/step-display';
  import { theme } from '../lib/theme.svelte';
  import type { BodyResult, DocumentPreview, ProfileFrame, ProfileLoop, ResultFaces } from '../lib/types';
  import {sectionPlaneOverlay} from '../lib/section-plane';
  import {claimSectionManipulator,configureSectionManipulator,sectionFrameFromPose,sectionManipulationPose,SectionManipulationGesture,type SectionManipulationMode} from '../lib/section-manipulation';
  import { SourceClickGesture, type SourceZone } from '../lib/selection';
  import { FacePaintStroke } from '../lib/face-paint';
  import { FaceClickSequence } from '../lib/region-selection';
  import { pickVisibleStepFace, pickVisibleSourceTriangle } from '../lib/source-picking';
  let { sectionPlane,sectionBounds,sectionMode,onsectiontransform,onsectiondrag,stepDisplay, bytes, results, preview, format, fitSignal, showEdges, unit, sourceRevision, selectionMode, zone, onpick, onmiss, stepPreview, stepTriangles, onfacepick, onfaceregion, onfacepaint, sourceHighlights }: {sectionPlane:ProfileFrame|null;sectionBounds:number[][]|null;sectionMode:SectionManipulationMode|null;onsectiontransform:(frame:ProfileFrame)=>void;onsectiondrag:(active:boolean,cancelled?:boolean)=>void; stepDisplay:StepDisplay|null; sourceHighlights:{ previewBodyId:string; triangles:number[] }[]|null; stepPreview:ResultFaces|null; stepTriangles:number[]; onfacepick:(face:number, additive:boolean)=>void; onfaceregion:(face:number)=>void; onfacepaint:(faces:number[])=>void; sourceRevision:number; selectionMode:boolean; zone:SourceZone | null; onpick:(bodyId:string, triangle:number) => void; onmiss:()=>void; unit: 'mm' | 'inch'; bytes: Uint8Array | null; results: BodyResult[]; preview: DocumentPreview | null; format: 'stl'|'3mf'; fitSignal: number; showEdges: boolean } = $props();
  let host: HTMLDivElement;
  let rebuild = () => {}; let rebuildEdges = () => {}; let rebuildSection = () => {};let updateSectionControls=()=>{};let updatePicking=()=>{};let highlight = () => {}; let recolor = () => {}; let fit = () => {};
  onMount(() => {
    const scene = new THREE.Scene();
    const camera = new THREE.PerspectiveCamera(38, 1, 0.001, 1e9);
    camera.up.set(0, 0, 1);
    const renderer = new THREE.WebGLRenderer({ antialias: true });
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2)); host.appendChild(renderer.domElement);
    const controls = new OrbitControls(camera, renderer.domElement);
    controls.enableDamping = false;
    const transform=new TransformControls(camera,renderer.domElement),planeAnchor=new THREE.Object3D();scene.add(planeAnchor);scene.add(transform.getHelper());transform.enabled=false;transform.setSpace('local');transform.setSize(0.85);
    const manipulation=new SectionManipulationGesture();let endingManipulation=false,cancellingManipulation=false,disposing=false,primaryPointer:number|null=null;
    let frame = 0;
    const draw = () => { if (!frame) frame = requestAnimationFrame(() => { frame = 0; renderer.render(scene, camera); }); };
    controls.addEventListener('change', draw);
    const ambient = new THREE.HemisphereLight(); scene.add(ambient);
    const directional = new THREE.DirectionalLight(); directional.position.set(2, -3, 5); scene.add(directional);
    let group = new THREE.Group(); scene.add(group);
    let edges = new THREE.Group(); scene.add(edges);
    let sectionGroup=new THREE.Group();scene.add(sectionGroup);
    const overlay = new THREE.Mesh(new THREE.BufferGeometry(), new THREE.MeshBasicMaterial({ transparent:true, opacity:0.65, side:THREE.DoubleSide, depthWrite:false, polygonOffset:true, polygonOffsetFactor:-2, polygonOffsetUnits:-2 })); scene.add(overlay);
    const meshes = new Map<string, THREE.Mesh>();
    let renderedRevision = -1, renderedNative = false, renderedStep:ResultFaces|null = null;
    let renderedBytes:Uint8Array|null=null, renderedUnit=unit, renderedFormat=format;
    let rawGeneration=0, denseRaw=false;
    let rawPreview:ReturnType<typeof createStlPreviewWorker>|null=null;
    const gesture = new SourceClickGesture();
    const raycaster = new THREE.Raycaster();
    function finishManipulation(cancelled:boolean,notify=true){
      if(endingManipulation)return;
      const started=manipulation.dragging,pointer=manipulation.pointer;
      endingManipulation=true;
      if(cancelled&&transform.dragging){cancellingManipulation=true;transform.reset();cancellingManipulation=false;}
      transform.pointerUp(null);transform.disconnect();
      if(pointer!==null&&renderer.domElement.hasPointerCapture(pointer))renderer.domElement.releasePointerCapture(pointer);
      if(!disposing)transform.connect(renderer.domElement);manipulation.finish();primaryPointer=null;gesture.cancel();controls.enabled=true;
      if(started&&notify)onsectiondrag(false,cancelled);
      endingManipulation=false;if(!disposing){updateSectionControls();draw();}
    }
    const claimManipulation=(event:PointerEvent)=>{
      if(!sectionMode||!transform.enabled)return;
      const claim=claimSectionManipulator(transform,manipulation,event,renderer.domElement.getBoundingClientRect());
      if(claim==='ignored'){event.stopImmediatePropagation();return;}
      if(event.button!==0)return;
      primaryPointer=event.pointerId;
      // Hover can be stale, or absent for touch/pen. Resolve the actual hit before Orbit's pointerdown.
      if(claim==='claimed'){gesture.cancel();controls.enabled=false;}
    };
    const transformDown=()=>{if(primaryPointer!==null)manipulation.claim(primaryPointer);manipulation.begin();gesture.cancel();controls.enabled=false;onsectiondrag(true);};
    const transformUp=()=>{if(!endingManipulation)finishManipulation(false);};
    const transformChanged=()=>{if(manipulation.dragging&&!cancellingManipulation){const frame=sectionFrameFromPose(planeAnchor.position,planeAnchor.quaternion);if(frame)onsectiontransform(frame);}draw();};
    transform.addEventListener('mouseDown',transformDown);transform.addEventListener('mouseUp',transformUp);transform.addEventListener('objectChange',transformChanged);transform.addEventListener('change',draw);
    const cancelTransformKey=(event:KeyboardEvent)=>{if(event.key==='Escape'&&manipulation.blocksPicking()){event.preventDefault();event.stopImmediatePropagation();finishManipulation(true);}};
    const guardOtherPointer=(event:PointerEvent)=>{if(manipulation.pointer!==null&&manipulation.pointer!==event.pointerId)event.stopImmediatePropagation();};
    const lostCapture=(event:PointerEvent)=>{queueMicrotask(()=>{if(!disposing&&manipulation.pointer===event.pointerId&&manipulation.blocksPicking())finishManipulation(true);});};
    renderer.domElement.addEventListener('pointerdown',claimManipulation,true);for(const type of ['pointermove','pointerup','pointercancel'])renderer.domElement.addEventListener(type,guardOtherPointer as EventListener,true);renderer.domElement.addEventListener('lostpointercapture',lostCapture);window.addEventListener('keydown',cancelTransformKey,true);
    const clicks = new FaceClickSequence();
    const paint=new FacePaintStroke();let paintFrame=0,paintDocument:ResultFaces|null=null;
    function hitFace(x:number,y:number){
      const rect=renderer.domElement.getBoundingClientRect();
      if(!rect.width||!rect.height||x<rect.left||x>rect.right||y<rect.top||y>rect.bottom)return null;
      return pickVisibleStepFace(raycaster,[...meshes.values()],new THREE.Vector2((x-rect.left)/rect.width*2-1,-(y-rect.top)/rect.height*2+1),camera);
    }
    function flushPaint(){
      if(paintFrame){cancelAnimationFrame(paintFrame);paintFrame=0;}
      if(paintDocument!==stepPreview||!selectionMode||sectionMode){finishPaint(false);return;}
      const faces=paint.sample(hitFace);if(faces.length)onfacepaint(faces);
    }
    function finishPaint(flush:boolean){
      if(flush)flushPaint();
      if(paintFrame){cancelAnimationFrame(paintFrame);paintFrame=0;}
      const pointer=paint.pointer,dragged=paint.finish();paintDocument=null;
      if(pointer!==null&&renderer.domElement.hasPointerCapture(pointer))renderer.domElement.releasePointerCapture(pointer);
      if(!manipulation.blocksPicking())controls.enabled=true;
      return dragged;
    }
    const paintDown=(e:PointerEvent)=>{
      if(paint.pointer!==null){finishPaint(false);gesture.cancel();clicks.clear();}
      if(!selectionMode||!stepPreview||sectionMode||manipulation.blocksPicking()||e.button!==0||e.isPrimary===false||e.altKey)return;
      gesture.down(e,true);paint.begin(e.pointerId,e.clientX,e.clientY);paintDocument=stepPreview;
      controls.enabled=false;renderer.domElement.setPointerCapture(e.pointerId);
      e.preventDefault();e.stopImmediatePropagation();
    };
    const paintLost=(e:PointerEvent)=>{if(paint.pointer===e.pointerId){finishPaint(false);gesture.cancel();clicks.clear();}};
    updatePicking=()=>{
      finishPaint(false);gesture.cancel();clicks.clear();
      const selecting=selectionMode&&!!stepPreview&&!sectionMode;
      controls.mouseButtons.LEFT=THREE.MOUSE.ROTATE;
      controls.mouseButtons.RIGHT=selecting?THREE.MOUSE.ROTATE:THREE.MOUSE.PAN;
      controls.mouseButtons.MIDDLE=selecting?THREE.MOUSE.PAN:THREE.MOUSE.DOLLY;
    };
    const cancelPaintKey=(e:KeyboardEvent)=>{if(e.key==='Escape'&&paint.pointer!==null){e.preventDefault();finishPaint(false);gesture.cancel();clicks.clear();}};
    window.addEventListener('keydown',cancelPaintKey,true);
    renderer.domElement.addEventListener('pointerdown',paintDown,true);
    renderer.domElement.addEventListener('lostpointercapture',paintLost);
    const pointerDown = (e:PointerEvent) => {if(!manipulation.blocksPicking())gesture.down(e,!!stepPreview);};
    const pointerMove = (e:PointerEvent) => {
      gesture.move(e);
      if(e.pointerId===paint.pointer){
        if(!(e.buttons&1)){finishPaint(false);gesture.cancel();return;}
        if(paint.move(e.pointerId,e.clientX,e.clientY)){clicks.clear();if(!paintFrame)paintFrame=requestAnimationFrame(()=>{paintFrame=0;flushPaint();});}
      }
    };
    const pointerCancel = () => {finishPaint(false);clicks.clear();gesture.cancel();if(manipulation.blocksPicking())finishManipulation(true);};
    const pointerUp = (e:PointerEvent) => {
      if(paint.pointer===e.pointerId){paint.move(e.pointerId,e.clientX,e.clientY);if(finishPaint(true)){gesture.cancel();clicks.clear();return;}}
      if(manipulation.blocksPicking()){gesture.cancel();queueMicrotask(()=>{if(manipulation.blocksPicking())finishManipulation(false);});return;}
      if (!gesture.up(e,!!stepPreview) || !selectionMode) {clicks.clear();return;}
      const rect = renderer.domElement.getBoundingClientRect();
      if (!rect.width || !rect.height) return;
      const pointer = new THREE.Vector2((e.clientX-rect.left)/rect.width*2-1, -(e.clientY-rect.top)/rect.height*2+1);
      if (stepPreview) { const face = pickVisibleStepFace(raycaster,[...meshes.values()],pointer,camera); if (face !== null) { if(clicks.click(face,stepPreview,e.timeStamp)==='region')onfaceregion(face);else onfacepick(face,e.shiftKey || e.metaKey || e.ctrlKey); } else {clicks.clear();onmiss();} return; }
      const hit = pickVisibleSourceTriangle(raycaster,[...meshes.values()],pointer,camera);
      if (hit) onpick(hit.bodyId,hit.triangle); else onmiss();
    };
    renderer.domElement.addEventListener('pointerdown',pointerDown);
    renderer.domElement.addEventListener('pointermove',pointerMove);
    renderer.domElement.addEventListener('pointerup',pointerUp);
    renderer.domElement.addEventListener('pointercancel',pointerCancel);
    window.addEventListener('blur',pointerCancel);
    const material = new THREE.MeshStandardMaterial({ roughness: 0.8, metalness: 0.05, side: THREE.DoubleSide });
    const stepMaterial = new THREE.MeshStandardMaterial({ vertexColors:true, roughness:0.9, metalness:0, side:THREE.DoubleSide, polygonOffset:true, polygonOffsetFactor:1, polygonOffsetUnits:1 });
    const lineMaterial = new THREE.LineBasicMaterial();
    const stepLineMaterial = new THREE.LineBasicMaterial({depthTest:true,depthWrite:false});
    const colorStep = (css:CSSStyleDeclaration) => {
      const geometry = meshes.get('step')?.geometry;
      if (!geometry || !stepPreview || !stepDisplay) return;
      const palette = Array.from({length:STEP_PALETTE_SIZE},(_,i) => new THREE.Color(css.getPropertyValue(`--view-face-${i+1}`).trim()));
      const size = geometry.getAttribute('position').count * 3;
      const previous = geometry.getAttribute('color');
      const attribute = previous instanceof THREE.BufferAttribute && previous.array instanceof Float32Array && previous.array.length === size ? previous : new THREE.BufferAttribute(new Float32Array(size),3);
      const colors = attribute.array;
      for (const face of stepPreview.faces) {
        const color = palette[stepDisplay.faceSlots[face.id]];
        for (let i = face.triangle_start * 9; i < (face.triangle_start + face.triangle_count) * 9; i += 3) { colors[i] = color.r; colors[i+1] = color.g; colors[i+2] = color.b; }
      }
      if (attribute !== previous) geometry.setAttribute('color',attribute);
      attribute.needsUpdate = true;
    };
    recolor = () => { const css = getComputedStyle(document.documentElement); const color = (key: string) => css.getPropertyValue(key).trim(); scene.background = new THREE.Color(color('--view-bg')); material.color.set(color('--view-mesh')); lineMaterial.color.set(color('--view-edge')); stepLineMaterial.color.set(color('--view-face-edge')); colorStep(css); overlay.material.color.set(color('--accent')); overlay.material.opacity = stepPreview ? 0.9 : 0.65;  ambient.color.set(color('--view-light')); ambient.groundColor.set(color('--view-ground')); directional.color.set(color('--view-light')); draw(); };
    fit = () => { const box = new THREE.Box3().setFromObject(group); if (box.isEmpty()) return; const center = box.getCenter(new THREE.Vector3()); const size = box.getSize(new THREE.Vector3()); const radius = Math.max(size.length() / 2, 0.01); controls.target.copy(center); camera.position.copy(center).add(new THREE.Vector3(1.5, -2, 1.4).normalize().multiplyScalar(radius / Math.sin(THREE.MathUtils.degToRad(camera.fov / 2)) * 1.15)); camera.near = radius / 1000; camera.far = radius * 1000; camera.updateProjectionMatrix(); controls.update(); draw(); };
    const disposeGroup = (target:THREE.Group) => target.traverse(o => { if (o instanceof THREE.Mesh || o instanceof THREE.LineSegments) o.geometry.dispose(); });
    const disposeSection=()=>sectionGroup.traverse(object=>{if(object instanceof THREE.Mesh||object instanceof THREE.Line){object.geometry.dispose();for(const material of Array.isArray(object.material)?object.material:[object.material])material.dispose();}});
    updateSectionControls=()=>{
      if(!sectionPlane||!sectionBounds||!sectionMode){if(manipulation.blocksPicking())finishManipulation(true);transform.detach();transform.enabled=false;return;}
      if(manipulation.dragging)return;
      const pose=sectionManipulationPose(sectionPlane,sectionBounds);if(!pose){transform.detach();return;}
      planeAnchor.position.copy(pose.position);planeAnchor.quaternion.copy(pose.quaternion);planeAnchor.scale.setScalar(1);planeAnchor.updateMatrixWorld();
      transform.attach(planeAnchor);transform.enabled=true;configureSectionManipulator(transform,sectionMode);
      const css=getComputedStyle(document.documentElement),color=(name:string)=>css.getPropertyValue(name).trim();
      transform.getHelper().traverse(object=>{if(object instanceof THREE.Mesh||object instanceof THREE.Line)for(const material of Array.isArray(object.material)?object.material:[object.material])if('color'in material){const colored=material as THREE.MeshBasicMaterial&{_color?:THREE.Color};colored.color.set(color('--text-secondary'));colored._color?.set(color('--text-secondary'));}});
      transform.setColors(color('--info'),color('--success'),color('--accent'),color('--accent-hover'));draw();
    };
    rebuildSection=()=>{
      scene.remove(sectionGroup);disposeSection();sectionGroup=new THREE.Group();scene.add(sectionGroup);
      const plane=sectionPlane&&sectionBounds?sectionPlaneOverlay(sectionPlane,sectionBounds):null;
      if(plane){
        const color=getComputedStyle(document.documentElement).getPropertyValue('--accent').trim();
        const fill=new THREE.BufferGeometry();fill.setAttribute('position',new THREE.Float32BufferAttribute(plane.positions,3));
        const mesh=new THREE.Mesh(fill,new THREE.MeshBasicMaterial({color,transparent:true,opacity:0.12,side:THREE.DoubleSide,depthWrite:false}));mesh.renderOrder=2;sectionGroup.add(mesh);
        const outline=new THREE.BufferGeometry();outline.setAttribute('position',new THREE.Float32BufferAttribute(plane.outline,3));
        const lines=new THREE.LineSegments(outline,new THREE.LineBasicMaterial({color,transparent:true,opacity:0.8,depthTest:false,depthWrite:false}));lines.renderOrder=3;sectionGroup.add(lines);
        const arrow=new THREE.ArrowHelper(new THREE.Vector3(...plane.normal as [number,number,number]),new THREE.Vector3(...plane.center as [number,number,number]),plane.arrowLength,color,plane.arrowLength*0.25,plane.arrowLength*0.12);
        arrow.line.geometry=arrow.line.geometry.clone();arrow.cone.geometry=arrow.cone.geometry.clone();
        for(const object of [arrow.line,arrow.cone]){for(const material of Array.isArray(object.material)?object.material:[object.material])material.depthTest=false;object.renderOrder=3;}sectionGroup.add(arrow);
      }
      draw();
    };
    highlight = () => {
      const groups = stepPreview ? [{ previewBodyId:'step', triangles:stepTriangles }] : sourceHighlights ?? (zone ? [zone] : []);
      const positions:number[] = [];
      for (const selected of groups) {
        const source = meshes.get(selected.previewBodyId);
        if (!source) continue;
        const attribute = source.geometry.getAttribute('position');
        for (const triangle of selected.triangles) if (Number.isInteger(triangle) && triangle >= 0 && triangle*3+2 < attribute.count) {
          for (let corner=0;corner<3;corner++) { const vertex=triangle*3+corner; positions.push(attribute.getX(vertex),attribute.getY(vertex),attribute.getZ(vertex)); }
        }
      }
      overlay.geometry.dispose(); overlay.geometry = new THREE.BufferGeometry();
      overlay.geometry.setAttribute('position',new THREE.Float32BufferAttribute(positions,3));
      overlay.geometry.computeBoundingSphere(); draw();
    };
    rebuild = () => {
      // Les résultats et changements de sélection ne reconstruisent jamais les meshes source.
      if (renderedRevision === sourceRevision && renderedNative === !!preview && renderedStep === stepPreview && renderedBytes === bytes && renderedUnit === unit && renderedFormat === format) return;
      const shouldFit = renderedRevision !== sourceRevision || !meshes.size;
      const generation=++rawGeneration;rawPreview?.clear();
      renderedBytes=bytes;renderedUnit=unit;renderedFormat=format;
      denseRaw=!stepPreview&&!preview&&!!bytes&&largeStlTriangles(bytes,format)!==null;
      renderedRevision = sourceRevision; renderedNative = !!preview; renderedStep = stepPreview; finishPaint(false);gesture.cancel();clicks.clear();
      scene.remove(group); disposeGroup(group); group = new THREE.Group(); scene.add(group); meshes.clear();
      const addMesh = (id:string, geometry:THREE.BufferGeometry, sourceTriangles?:Uint32Array) => {
        if(!sourceTriangles)geometry.computeVertexNormals();
        const mesh = new THREE.Mesh(geometry,id === 'step' ? stepMaterial : material);
        mesh.userData.previewBodyId = id;
        mesh.userData.sourceTriangles = sourceTriangles??Uint32Array.from({ length:geometry.getAttribute('position').count/3 },(_,i) => i);
        meshes.set(id,mesh); group.add(mesh);
      };
      if (stepPreview) {
        const positions = stepPreview.mesh.triangles.flatMap(triangle => triangle.flatMap(index => stepPreview!.mesh.vertices[index]));
        const geo = new THREE.BufferGeometry(); geo.setAttribute('position',new THREE.Float32BufferAttribute(positions,3)); addMesh('step',geo);
        const owners = new Uint32Array(stepPreview.mesh.triangles.length);
        for (const face of stepPreview.faces) owners.fill(face.id,face.triangle_start,face.triangle_start+face.triangle_count);
        meshes.get('step')!.userData.stepFaces = owners;
      } else if (preview) { for (const body of preview.bodies) { const positions = body.mesh.triangles.flatMap(triangle => triangle.flatMap(index => body.mesh.vertices[index] ?? [])); if (positions.length) { const geo = new THREE.BufferGeometry(); geo.setAttribute('position', new THREE.Float32BufferAttribute(positions, 3)); addMesh(body.id,geo); } } }
      else if (bytes && format === 'stl') {
        if(denseRaw) {
          if(!rawPreview) {
            try {rawPreview=createStlPreviewWorker(new Worker(new URL('../lib/stl-preview.worker.ts',import.meta.url),{type:'module'}));}
            catch {/* A dense display never falls back to parsing on the UI thread. */}
          }
          const source=bytes,sourceUnit=unit,revision=sourceRevision;
          void rawPreview?.prepare(source,sourceUnit).then(prepared=>{
            if(disposing||generation!==rawGeneration||bytes!==source||unit!==sourceUnit||sourceRevision!==revision||preview||stepPreview||format!=='stl')return;
            const geometry=new THREE.BufferGeometry();
            geometry.setAttribute('position',new THREE.BufferAttribute(prepared.positions,3));
            geometry.setAttribute('normal',new THREE.BufferAttribute(prepared.normals,3));
            geometry.boundingBox=new THREE.Box3(new THREE.Vector3(...prepared.bounds[0] as [number,number,number]),new THREE.Vector3(...prepared.bounds[1] as [number,number,number]));
            geometry.boundingSphere=new THREE.Sphere(new THREE.Vector3(...prepared.sphere.center as [number,number,number]),prepared.sphere.radius);
            addMesh('stl',geometry,prepared.sourceTriangles);
            recolor();highlight();rebuildEdges();if(shouldFit)fit();draw();
          }).catch(()=>{/* Display refusal leaves the conversion and source intact. */});
        } else {
          try { const data = bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength) as ArrayBuffer; const geometry = new STLLoader().parse(data); if (unit === 'inch') geometry.scale(25.4, 25.4, 25.4); addMesh('stl',geometry); } catch { /* Le moteur explique les fichiers invalides. */ }
        }
      }
      recolor(); highlight(); if (shouldFit) fit(); draw();
    };
    rebuildEdges = () => {
      scene.remove(edges); disposeGroup(edges); edges = new THREE.Group(); scene.add(edges);
      if (stepPreview && stepDisplay && showEdges) {
        const geo = new THREE.BufferGeometry(); geo.setAttribute('position',new THREE.BufferAttribute(stepDisplay.boundaries,3));
        const contours = new THREE.LineSegments(geo,stepLineMaterial); contours.renderOrder = 1; edges.add(contours);
      } else if (!stepPreview && !denseRaw && results.length && showEdges) {
        const points: number[] = [];
        for (const result of results) {
        if (result.rounded_box_recognition) {
          // Show the source edges shared by distinct proven surface patches.
          const mesh = result.analysis.mesh;
          const owners = result.rounded_box_recognition.triangle_faces;
          const edges = new Map<string, { owner:string; a:number; b:number }>();
          mesh.triangles.forEach((triangle, index) => {
            const owner = JSON.stringify(owners[index]);
            for (let i = 0; i < 3; i++) {
              const a = triangle[i], b = triangle[(i + 1) % 3];
              const key = a < b ? `${a}:${b}` : `${b}:${a}`;
              const previous = edges.get(key);
              if (previous) {
                if (previous.owner !== owner) points.push(...mesh.vertices[a], ...mesh.vertices[b]);
                edges.delete(key);
              } else { edges.set(key, { owner, a, b }); }
            }
          });
        } else if (result.rounded_recognition || result.revolved_recognition || result.mixed_recognition) {
          const model = result.rounded_recognition?.model.revolved.mixed ?? result.revolved_recognition?.model.mixed ?? result.mixed_recognition!.model;
          for (const plane of model.planes) for (const loop of plane.loops) {
            if (loop.kind === 'polygon') {
              for (let i=0; i<loop.vertices.length; i++) {
                const a=model.vertices[loop.vertices[i]], b=model.vertices[loop.vertices[(i+1)%loop.vertices.length]];
                if (a && b) points.push(...a,...b);
              }
            } else if (loop.kind === 'segments') {
              for (const segment of loop.segments) if (segment.kind === 'line') {
                const a=model.vertices[segment.vertices[0]], b=model.vertices[segment.vertices[1]];
                if (a && b) points.push(...a,...b);
              }
            }
          }
          // Arcs tracés une fois chacun, du sommet de départ autour de l'axe.
          for (const arc of model.arcs ?? []) {
            const start=model.vertices[arc.vertices[0]];
            if (!start) continue;
            const center=new THREE.Vector3(...arc.center as [number,number,number]);
            const axis=new THREE.Vector3(...arc.axis as [number,number,number]).normalize();
            const radial=new THREE.Vector3(...start as [number,number,number]).sub(center).normalize().multiplyScalar(arc.radius_mm);
            const steps=Math.max(1, Math.ceil(128*Math.abs(arc.sweep_rad)/(2*Math.PI)));
            const vertex=(angle: number) => center.clone().add(radial.clone().applyAxisAngle(axis,angle)).toArray();
            for (let i=0;i<steps;i++) points.push(...vertex(arc.sweep_rad*i/steps),...vertex(arc.sweep_rad*(i+1)/steps));
          }
          for (const ring of model.rings) {
            if (ring.radius_mm === 0) continue;
            const center=new THREE.Vector3(...ring.center as [number,number,number]);
            const axis=new THREE.Vector3(...ring.axis as [number,number,number]).normalize();
            const seed=Math.abs(axis.x)<0.9 ? new THREE.Vector3(1,0,0) : new THREE.Vector3(0,1,0);
            const x=new THREE.Vector3().crossVectors(axis,seed).normalize();
            const y=new THREE.Vector3().crossVectors(axis,x);
            const vertex=(angle: number) => center.clone().addScaledVector(x,ring.radius_mm*Math.cos(angle)).addScaledVector(y,ring.radius_mm*Math.sin(angle)).toArray();
            for (let i=0;i<128;i++) points.push(...vertex(i*Math.PI/64),...vertex((i+1)*Math.PI/64));
          }
        } else if (result.recognition) {
          const e = result.recognition.extrusion;
          const origin = new THREE.Vector3(...e.origin as [number,number,number]);
          const axis = new THREE.Vector3(...e.axis as [number,number,number]);
          const x = new THREE.Vector3(...e.x_axis as [number,number,number]);
          const y = new THREE.Vector3().crossVectors(axis, x);
          const vertex = (a: number, b: number, h: number) => origin.clone().addScaledVector(x,a).addScaledVector(y,b).addScaledVector(axis,h).toArray();
          for (const loop of [e.outer, ...e.holes] as ProfileLoop[]) {
            const profile = loop.kind === 'circle'
              ? Array.from({length:128}, (_,i) => [loop.center[0]+loop.radius_mm*Math.cos(i*Math.PI/64), loop.center[1]+loop.radius_mm*Math.sin(i*Math.PI/64)])
              : loop.points;
            for (let i=0;i<profile.length;i++) for (const h of [0,e.height_mm]) {
              const a=profile[i], b=profile[(i+1)%profile.length]; points.push(...vertex(a[0],a[1],h), ...vertex(b[0],b[1],h));
            }
          }
        } else {
          const model = result.analysis.model;
          for (const face of model.faces) for (const loop of face.loops) for (let i = 0; i < loop.length; i++) { const a = model.vertices[loop[i]], b = model.vertices[loop[(i + 1) % loop.length]]; if (a && b) points.push(...a, ...b); }
        }
        }
        if (points.length) { const geo = new THREE.BufferGeometry(); geo.setAttribute('position', new THREE.Float32BufferAttribute(points, 3)); edges.add(new THREE.LineSegments(geo, lineMaterial)); }
      }
      draw();
    };
    const resize = new ResizeObserver(() => { const w = host.clientWidth, h = host.clientHeight; if (!w || !h) return; renderer.setSize(w, h); camera.aspect = w / h; camera.updateProjectionMatrix(); draw(); }); resize.observe(host);
    recolor(); rebuild(); rebuildEdges();
    return () => { disposing=true;rawGeneration++;rawPreview?.dispose();window.removeEventListener('keydown',cancelPaintKey,true);finishPaint(false);updatePicking=()=>{};renderer.domElement.removeEventListener('pointerdown',paintDown,true);renderer.domElement.removeEventListener('lostpointercapture',paintLost);if(manipulation.blocksPicking())finishManipulation(true,false);updateSectionControls=()=>{};transform.removeEventListener('mouseDown',transformDown);transform.removeEventListener('mouseUp',transformUp);transform.removeEventListener('objectChange',transformChanged);transform.removeEventListener('change',draw);transform.dispose();resize.disconnect(); controls.dispose();renderer.domElement.removeEventListener('pointerdown',claimManipulation,true);for(const type of ['pointermove','pointerup','pointercancel'])renderer.domElement.removeEventListener(type,guardOtherPointer as EventListener,true);renderer.domElement.removeEventListener('lostpointercapture',lostCapture);window.removeEventListener('keydown',cancelTransformKey,true); renderer.domElement.removeEventListener('pointerdown',pointerDown); renderer.domElement.removeEventListener('pointermove',pointerMove); renderer.domElement.removeEventListener('pointerup',pointerUp); renderer.domElement.removeEventListener('pointercancel',pointerCancel); window.removeEventListener('blur',pointerCancel); disposeGroup(edges);disposeSection();overlay.geometry.dispose(); overlay.material.dispose(); cancelAnimationFrame(frame); group.traverse(o => { if (o instanceof THREE.Mesh || o instanceof THREE.LineSegments) o.geometry.dispose(); }); material.dispose(); stepMaterial.dispose(); lineMaterial.dispose(); stepLineMaterial.dispose(); renderer.dispose(); renderer.domElement.remove(); };
  });
  $effect(() => { void bytes; void preview; void format; void unit; void sourceRevision; void stepPreview; rebuild(); });
  $effect(() => { void results; void showEdges; void stepPreview; void stepDisplay; rebuildEdges(); });
  $effect(() => { void zone; void sourceHighlights; void stepTriangles; void stepPreview; highlight(); });
  $effect(() => { void theme.applied; void stepDisplay; recolor(); });
  $effect(()=>{void sectionPlane;void sectionBounds;void sectionMode;void theme.applied;rebuildSection();updateSectionControls();});
  $effect(() => { void fitSignal; fit(); });
  $effect(()=>{void selectionMode;void stepPreview;void sourceRevision;void sectionMode;updatePicking();});
</script>
<div class="viewport" class:selecting={selectionMode&&!sectionMode} bind:this={host}></div>
<style>.viewport { position:absolute; inset:0; overflow:hidden; } .viewport.selecting :global(canvas) { cursor:crosshair; } .viewport :global(canvas) { display:block; width:100%; height:100%; }</style>
