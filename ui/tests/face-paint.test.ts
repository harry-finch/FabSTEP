import {test} from 'node:test';
import {strict as assert} from 'node:assert';
import {FacePaintStroke,FacePaintQueue} from '../src/lib/face-paint.ts';
const tick=()=>new Promise<void>(resolve=>setImmediate(resolve));

test('paint starts after click threshold, samples intervening visible faces and never toggles repeats',()=>{
  const stroke=new FacePaintStroke();
  assert.equal(stroke.begin(1,0,0),true);
  assert.equal(stroke.move(1,3,0),false);
  assert.deepEqual(stroke.sample(()=>5),[]);
  assert.equal(stroke.move(1,16,0),true);
  assert.deepEqual(stroke.sample(x=>Math.floor(x/4)),[0,1,2,3,4]);
  stroke.move(1,0,0);assert.deepEqual(stroke.sample(x=>Math.floor(x/4)),[]);
  assert.equal(stroke.finish(),true);
  stroke.begin(2,0,0);stroke.move(2,6,0);assert.deepEqual(stroke.sample(()=>0),[0]);
});
test('paint ray sampling is bounded and never selects through a miss',()=>{
  const stroke=new FacePaintStroke();stroke.begin(1,0,0);stroke.move(1,1e8,1e8);
  let calls=0;assert.deepEqual(stroke.sample(()=>{calls++;return null;}),[]);
  assert.equal(calls,64);
});
test('cancel, pointer identity and fresh strokes cannot emit abandoned pending points',()=>{
  const stroke=new FacePaintStroke();stroke.begin(1,0,0);
  assert.equal(stroke.begin(2,0,0),false);assert.equal(stroke.move(2,20,0),false);
  stroke.move(1,20,0);stroke.cancel();assert.deepEqual(stroke.sample(()=>4),[]);assert.equal(stroke.pointer,null);
  stroke.begin(3,1,1);assert.equal(stroke.finish(),false);
});
test('slow worker keeps one batch active and unions all later painted faces',async()=>{
  const queue=new FacePaintQueue(),sent:number[][]=[];let ack:(()=>void)|null=null;
  const send=(ids:number[])=>{sent.push(ids);return new Promise<void>(resolve=>ack=resolve);};
  queue.add([1],send);queue.add([2,3],send);queue.add([3,4],send);
  assert.deepEqual(sent,[[1]]);ack!();await tick();assert.deepEqual(sent,[[1],[2,3,4]]);
  ack!();await tick();queue.add([4],async ids=>{sent.push(ids);});await tick();assert.deepEqual(sent.at(-1),[4]);
});
test('clear discards pending faces while stale ack releases queue for new context',async()=>{
  const queue=new FacePaintQueue(),sent:number[][]=[];let ack:(()=>void)|null=null;
  queue.add([1],ids=>{sent.push(ids);return new Promise<void>(resolve=>ack=resolve);});
  queue.add([2],async ids=>{sent.push(ids);});queue.clear();
  queue.add([9],async ids=>{sent.push(ids);});assert.deepEqual(sent,[[1]]);
  ack!();await tick();assert.deepEqual(sent,[[1],[9]]);
});
test('failed worker dispatch clears pending work and a later stroke can recover',async()=>{
  const queue=new FacePaintQueue(),sent:number[][]=[];
  queue.add([1],async()=>{throw new Error('closed');});queue.add([2],async ids=>{sent.push(ids);});
  await tick();assert.deepEqual(sent,[]);queue.add([3],async ids=>{sent.push(ids);});await tick();assert.deepEqual(sent,[[3]]);
});
