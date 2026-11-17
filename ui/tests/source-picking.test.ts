import { test } from 'node:test';
import { strict as assert } from 'node:assert';
import * as THREE from 'three';
import { pickVisibleSourceTriangle } from '../src/lib/source-picking.ts';

test('raycast selects the nearest source occurrence regardless of body list order and ignores empty space', () => {
  const camera = new THREE.PerspectiveCamera(45,1,0.1,100); camera.position.set(0,0,10); camera.lookAt(0,0,0);
  const material = new THREE.MeshBasicMaterial({ side:THREE.DoubleSide });
  const front = new THREE.Mesh(new THREE.PlaneGeometry(2,2),material), back = new THREE.Mesh(new THREE.PlaneGeometry(2,2),material);
  front.position.z=2;
  front.userData = { previewBodyId:'3mf/occurrence/front',sourceTriangles:new Uint32Array([7,8]) };
  back.userData = { previewBodyId:'3mf/occurrence/back',sourceTriangles:new Uint32Array([2,3]) };
  const ray = new THREE.Raycaster();
  assert.equal(pickVisibleSourceTriangle(ray,[back,front],new THREE.Vector2(0.1,0.1),camera)?.bodyId,'3mf/occurrence/front');
  assert.ok([7,8].includes(pickVisibleSourceTriangle(ray,[front,back],new THREE.Vector2(0.1,0.1),camera)!.triangle));
  assert.equal(pickVisibleSourceTriangle(ray,[back,front],new THREE.Vector2(0.9,0.9),camera),null);
  front.geometry.dispose(); back.geometry.dispose(); material.dispose();
});

test('STEP raycast returns native whole-face ownership, never source triangle identity', async () => {
  const { pickVisibleStepFace } = await import('../src/lib/source-picking.ts');
  const camera = new THREE.PerspectiveCamera(45,1,0.1,100); camera.position.set(0,0,10); camera.lookAt(0,0,0);
  const material = new THREE.MeshBasicMaterial({side:THREE.DoubleSide});
  const front = new THREE.Mesh(new THREE.PlaneGeometry(2,2),material), back = new THREE.Mesh(new THREE.PlaneGeometry(2,2),material);
  front.position.z=2; front.userData={stepFaces:new Uint32Array([19,19]),sourceTriangles:new Uint32Array([5,6])}; back.userData={stepFaces:new Uint32Array([4,4])};
  const ray = new THREE.Raycaster();
  assert.equal(pickVisibleStepFace(ray,[back,front],new THREE.Vector2(.1,.1),camera),19);
  assert.equal(pickVisibleStepFace(ray,[front,back],new THREE.Vector2(-.1,-.1),camera),19);
  assert.equal(pickVisibleStepFace(ray,[back,front],new THREE.Vector2(.9,.9),camera),null);
  front.geometry.dispose();back.geometry.dispose();material.dispose();
});
