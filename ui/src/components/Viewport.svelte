<script lang="ts">
  // Vue 3D à la demande : orbite, zoom, cadrage et contours des faces reconnues.
  import { onMount } from 'svelte';
  import * as THREE from 'three';
  import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
  import { STLLoader } from 'three/examples/jsm/loaders/STLLoader.js';
  import { theme } from '../lib/theme.svelte';
  import type { BodyResult, DocumentPreview, ProfileLoop } from '../lib/types';
  let { bytes, results, preview, format, fitSignal, showEdges, unit }: { unit: 'mm' | 'inch'; bytes: Uint8Array | null; results: BodyResult[]; preview: DocumentPreview | null; format: 'stl'|'3mf'; fitSignal: number; showEdges: boolean } = $props();
  let host: HTMLDivElement;
  let rebuild = () => {}; let recolor = () => {}; let fit = () => {};
  onMount(() => {
    const scene = new THREE.Scene();
    const camera = new THREE.PerspectiveCamera(38, 1, 0.001, 1e9);
    camera.up.set(0, 0, 1);
    const renderer = new THREE.WebGLRenderer({ antialias: true });
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2)); host.appendChild(renderer.domElement);
    const controls = new OrbitControls(camera, renderer.domElement);
    controls.enableDamping = false;
    let frame = 0;
    const draw = () => { if (!frame) frame = requestAnimationFrame(() => { frame = 0; renderer.render(scene, camera); }); };
    controls.addEventListener('change', draw);
    const ambient = new THREE.HemisphereLight(); scene.add(ambient);
    const directional = new THREE.DirectionalLight(); directional.position.set(2, -3, 5); scene.add(directional);
    let group = new THREE.Group(); scene.add(group);
    const material = new THREE.MeshStandardMaterial({ roughness: 0.8, metalness: 0.05, side: THREE.DoubleSide });
    const lineMaterial = new THREE.LineBasicMaterial();
    recolor = () => { const css = getComputedStyle(document.documentElement); const color = (key: string) => css.getPropertyValue(key).trim(); scene.background = new THREE.Color(color('--view-bg')); material.color.set(color('--view-mesh')); lineMaterial.color.set(color('--view-edge')); ambient.color.set(color('--view-light')); ambient.groundColor.set(color('--view-ground')); directional.color.set(color('--view-light')); draw(); };
    fit = () => { const box = new THREE.Box3().setFromObject(group); if (box.isEmpty()) return; const center = box.getCenter(new THREE.Vector3()); const size = box.getSize(new THREE.Vector3()); const radius = Math.max(size.length() / 2, 0.01); controls.target.copy(center); camera.position.copy(center).add(new THREE.Vector3(1.5, -2, 1.4).normalize().multiplyScalar(radius / Math.sin(THREE.MathUtils.degToRad(camera.fov / 2)) * 1.15)); camera.near = radius / 1000; camera.far = radius * 1000; camera.updateProjectionMatrix(); controls.update(); draw(); };
    rebuild = () => {
      scene.remove(group); group.traverse(o => { if (o instanceof THREE.Mesh || o instanceof THREE.LineSegments) o.geometry.dispose(); }); group = new THREE.Group(); scene.add(group);
      if (preview) { for (const body of preview.bodies) { const positions = body.mesh.triangles.flatMap(triangle => triangle.flatMap(index => body.mesh.vertices[index] ?? [])); if (positions.length) { const geo = new THREE.BufferGeometry(); geo.setAttribute('position', new THREE.Float32BufferAttribute(positions, 3)); geo.computeVertexNormals(); group.add(new THREE.Mesh(geo, material)); } } }
      else if (bytes && format === 'stl') { try { const data = bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength) as ArrayBuffer; const geometry = new STLLoader().parse(data); geometry.computeVertexNormals(); if (unit === 'inch') geometry.scale(25.4, 25.4, 25.4); group.add(new THREE.Mesh(geometry, material)); } catch { /* Le moteur explique les fichiers invalides. */ } }
      if (results.length && showEdges) {
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
        if (points.length) { const geo = new THREE.BufferGeometry(); geo.setAttribute('position', new THREE.Float32BufferAttribute(points, 3)); group.add(new THREE.LineSegments(geo, lineMaterial)); }
      }
      fit(); draw();
    };
    const resize = new ResizeObserver(() => { const w = host.clientWidth, h = host.clientHeight; if (!w || !h) return; renderer.setSize(w, h); camera.aspect = w / h; camera.updateProjectionMatrix(); draw(); }); resize.observe(host);
    recolor(); rebuild();
    return () => { resize.disconnect(); controls.dispose(); cancelAnimationFrame(frame); group.traverse(o => { if (o instanceof THREE.Mesh || o instanceof THREE.LineSegments) o.geometry.dispose(); }); material.dispose(); lineMaterial.dispose(); renderer.dispose(); renderer.domElement.remove(); };
  });
  $effect(() => { void bytes; void results; void preview; void format; void showEdges; void unit; rebuild(); });
  $effect(() => { void theme.applied; recolor(); });
  $effect(() => { void fitSignal; fit(); });
</script>
<div class="viewport" bind:this={host}></div>
<style>.viewport { position:absolute; inset:0; overflow:hidden; } .viewport :global(canvas) { display:block; width:100%; height:100%; }</style>
