import { SelectionGraph, type Attribution, type SelectionToken } from './selection.ts';
import type { SelectionTopology } from './types.ts';

type Message = { token:SelectionToken } & (
  { type:'prepare'; topology:SelectionTopology; bodyCounts:Record<string,number> } |
  { type:'interpret'; items:Attribution[] } |
  { type:'inspect'; bodyId:string; triangles:number[] } |
  { type:'zone'; bodyId:string; seed:number; triangles:number[]; ring:number }
);
let graph = new SelectionGraph();
let revision = -1, requestId = -1;
self.onmessage = (event:MessageEvent<Message>) => {
  const message = event.data, { token } = message;
  try {
    if (message.type === 'prepare') {
      graph = new SelectionGraph(); graph.prepare(message.topology,message.bodyCounts);
      revision = token.revision; requestId = token.requestId;
      self.postMessage({ type:'ready', token }); return;
    }
    if (token.revision !== revision || token.requestId !== requestId) return;
    if (message.type === 'interpret') { graph.interpret(message.items); self.postMessage({ type:'interpreted', token }); return; }
    if (message.type === 'inspect') self.postMessage({ type:'inspection', token, inspection:graph.inspect(message.bodyId,message.triangles) });
    else self.postMessage({ type:'zone', token, ...graph.zone(message.bodyId,message.seed,message.triangles,message.ring,token.epoch) });
  } catch {
    if (message.type === 'prepare') { revision = -1; requestId = -1; }
    self.postMessage({ type:message.type === 'prepare' ? 'unavailable' : 'error', token });
  }
};
