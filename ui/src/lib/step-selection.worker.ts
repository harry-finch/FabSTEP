import { prepareStepDisplay } from './step-display.ts';
import { StepSelectionGraph } from './step-selection.ts';
import type { SelectionToken } from './selection.ts';
import type { ResultFaces } from './types.ts';
type Message = { token:SelectionToken } & ({ type:'prepare'; document:ResultFaces; bodyCounts?:Record<string,number> }|{ type:'pick'; id:number; additive:boolean }|{ type:'select'|'add'; ids:number[] }|{ type:'clear'|'grow'|'shrink' });
let graph = new StepSelectionGraph(), revision = -1, requestId = -1;
self.onmessage = (event:MessageEvent<Message>) => {
  const message = event.data, { token } = message;
  try {
    if (message.type === 'prepare') {
      graph = new StepSelectionGraph(); graph.prepare(message.document,message.bodyCounts);
      revision = token.revision; requestId = token.requestId;
      const display = prepareStepDisplay(message.document);
      self.postMessage({ type:'ready', token, selection:graph.inspect(), display }, {transfer:[display.faceSlots.buffer,display.boundaries.buffer]}); return;
    }
    if (token.revision !== revision || token.requestId !== requestId) return;
    const selection = message.type === 'pick' ? graph.pick(message.id,message.additive) : message.type === 'select' || message.type === 'add' ? graph[message.type](message.ids) : graph[message.type]();
    self.postMessage({ type:'selection', token, selection });
  } catch {
    if (message.type === 'prepare') { revision = -1; requestId = -1; }
    self.postMessage({ type:'error', token });
  }
};
