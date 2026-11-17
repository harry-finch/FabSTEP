import { STLLoader } from 'three/examples/jsm/loaders/STLLoader.js';
import { largeStlTriangles } from './large-stl.ts';
import type { StlPreviewGeometry } from './stl-preview-worker.ts';
self.onmessage=(event:MessageEvent<{id:number;buffer:ArrayBuffer;unit:'mm'|'inch'}>)=>{
  const {id,buffer,unit}=event.data;
  try {
    if(!(buffer instanceof ArrayBuffer)||!Number.isSafeInteger(id)||id<1||(unit!=='mm'&&unit!=='inch'))throw new Error('Invalid display request.');
    const count=largeStlTriangles(new Uint8Array(buffer),'stl');
    if(count===null)throw new Error('Invalid dense binary STL framing.');
    const geometry=new STLLoader().parse(buffer);
    try {
      if(unit==='inch')geometry.scale(25.4,25.4,25.4);
      const positions=geometry.getAttribute('position').array;
      if(!(positions instanceof Float32Array)||positions.length!==count*9)throw new Error('Invalid display mesh.');
      for(const value of positions)if(!Number.isFinite(value)||Math.abs(value)>1e9)throw new Error('Source coordinates are outside display limits.');
      geometry.computeVertexNormals();geometry.computeBoundingBox();geometry.computeBoundingSphere();
      const normals=geometry.getAttribute('normal').array;
      if(!(normals instanceof Float32Array)||!geometry.boundingBox||!geometry.boundingSphere||!Number.isFinite(geometry.boundingSphere.radius))throw new Error('Invalid display bounds.');
      const sourceTriangles=new Uint32Array(count);for(let i=0;i<count;i++)sourceTriangles[i]=i;
      const result:StlPreviewGeometry={positions,normals,sourceTriangles,bounds:[geometry.boundingBox.min.toArray(),geometry.boundingBox.max.toArray()],sphere:{center:geometry.boundingSphere.center.toArray(),radius:geometry.boundingSphere.radius}};
      self.postMessage({id,geometry:result},{transfer:[positions.buffer,normals.buffer,sourceTriangles.buffer]});
    } finally {geometry.dispose();}
  } catch {
    self.postMessage({id,error:{code:'preview.invalid_stl',message:'Dense binary STL display is unavailable.'}});
  }
};
