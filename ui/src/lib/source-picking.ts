import * as THREE from 'three';

/** Le rayon voit toutes les occurrences source ; les overlays ne sont pas des cibles. */
export function pickVisibleSourceTriangle(raycaster:THREE.Raycaster, meshes:THREE.Mesh[], pointer:THREE.Vector2, camera:THREE.Camera):{ bodyId:string; triangle:number } | null {
  camera.updateMatrixWorld();
  for (const mesh of meshes) mesh.updateMatrixWorld();
  raycaster.setFromCamera(pointer,camera);
  const hit = raycaster.intersectObjects(meshes,false)[0];
  if (!hit || hit.faceIndex == null) return null;
  const triangle = (hit.object.userData.sourceTriangles as Uint32Array | undefined)?.[hit.faceIndex];
  const bodyId:unknown = hit.object.userData.previewBodyId;
  return triangle !== undefined && typeof bodyId === 'string' ? { bodyId,triangle } : null;
}

/** L'appartenance native est fournie par la tessellation, jamais par le maillage source. */
export function pickVisibleStepFace(raycaster:THREE.Raycaster, meshes:THREE.Mesh[], pointer:THREE.Vector2, camera:THREE.Camera):number | null {
  camera.updateMatrixWorld();
  for (const mesh of meshes) mesh.updateMatrixWorld();
  raycaster.setFromCamera(pointer,camera);
  const hit = raycaster.intersectObjects(meshes,false)[0];
  if (!hit || hit.faceIndex == null) return null;
  return (hit.object.userData.stepFaces as Uint32Array | undefined)?.[hit.faceIndex] ?? null;
}
