import { faceProfileGeometry, sectionProfileGeometry } from './face-profile-geometry.ts';
import type { PlanarFaceProfile, SectionProfile } from './types.ts';
self.onmessage = (event:MessageEvent<{ id:number } & ({kind?:'face';profile:PlanarFaceProfile}|{kind:'section';profile:SectionProfile})>) => {
  const message = event.data, {id} = message;
  try { self.postMessage({ id, geometry:message.kind === 'section' ? sectionProfileGeometry(message.profile) : faceProfileGeometry(message.profile) }); }
  catch (error) { self.postMessage({ id, error }); }
};
