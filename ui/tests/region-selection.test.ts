import { test } from 'node:test';
import { strict as assert } from 'node:assert';
import { FaceClickSequence, requestSuggestedRegion, RegionSelectionQueue } from '../src/lib/region-selection.ts';
import { SelectionReplies } from '../src/lib/selection.ts';
import { restoreOptions } from '../src/lib/options-storage.ts';
import type { ResultFaces,Options } from '../src/lib/types.ts';
const document={request_id:7,source_id:2,faces:[{id:0,solid_id:0},{id:1,solid_id:0},{id:2,solid_id:1}]} as ResultFaces;
const deferred=<T>()=>{let resolve!:(v:T)=>void, reject!:(v:unknown)=>void;const promise=new Promise<T>((a,b)=>{resolve=a;reject=b;});return {promise,resolve,reject};};

test('double click requests one wall without a second face toggle; orbit and context reset it',()=>{
 const clicks=new FaceClickSequence();
 assert.equal(clicks.click(1,document,100),'face');assert.equal(clicks.click(1,document,200),'region');
 assert.equal(clicks.click(1,document,250),'face');
 clicks.clear();assert.equal(clicks.click(1,document,300),'face');
 assert.equal(clicks.click(1,{},350),'face');
 assert.equal(clicks.click(0,document,400),'face');assert.equal(clicks.click(0,document,1000),'face');
});

test('wall suggestion is read-only, validates identity, and only newest selection wins',async()=>{
 const replies=new SelectionReplies(), first=deferred<any>(),second=deferred<any>();const calls:string[]=[];
 const bridge={invoke:async<T>(command:string,args:Record<string,unknown>)=>{
  calls.push(command);assert.deepEqual(args,{requestId:7,faceId:0});return (calls.length===1?first.promise:second.promise) as Promise<T>;
 }};
 const a=replies.next(7);const p=requestSuggestedRegion(bridge,document,0,()=>replies.accepts(a,7));
 const b=replies.next(7);const q=requestSuggestedRegion(bridge,document,0,()=>replies.accepts(b,7));
 first.resolve({request_id:7,face_ids:[0,1]});second.resolve({request_id:7,face_ids:[1,0]});
 assert.equal(await p,null);assert.deepEqual(await q,[0,1]);assert.deepEqual(calls,['suggest_region','suggest_region']);
});

test('view/source changes ignore late region and malformed responses never change selection',async()=>{
 for(const invalidate of ['clear','newSource'] as const){
  const replies=new SelectionReplies(),token=replies.next(7),result=deferred<any>();
  const p=requestSuggestedRegion({invoke:()=>result.promise},document,0,()=>replies.accepts(token,7));
  replies[invalidate]();result.resolve({request_id:7,face_ids:[0,1]});assert.equal(await p,null);
 }
 for(const response of [{request_id:8,face_ids:[0]},{request_id:7,face_ids:[]},{request_id:7,face_ids:[1]},{request_id:7,face_ids:[0,0]},{request_id:7,face_ids:[0,2]},{request_id:7,face_ids:[0,9]}]){
  await assert.rejects(requestSuggestedRegion({invoke:async()=>response as any},document,0,()=>true),/selection.invalid_region/);
 }
});

test('preference migration defaults to Auto once, preserves tolerances and later explicit strategy',()=>{
 const initial:Options={input_unit:'mm',reconstruction:'auto',weld_tolerance_mm:1e-6,plane_tolerance_mm:1e-6,cylinder_fit_tolerance_mm:1e-5,max_deviation_mm:0.05};
 const old={...initial,input_unit:'inch',reconstruction:'surfaces',plane_tolerance_mm:1e-7};
 assert.deepEqual(restoreOptions(initial,null,JSON.stringify(old)),{...old,reconstruction:'auto'});
 assert.deepEqual(restoreOptions(initial,JSON.stringify(old),null),old);
 const relief:Options={...initial,reconstruction:'height_patch'};
 assert.deepEqual(restoreOptions(initial,JSON.stringify(relief),null),relief);
 assert.equal(restoreOptions(initial,null,JSON.stringify(relief)).reconstruction,'auto');
 assert.deepEqual(restoreOptions(initial,'{',null),initial);
 assert.deepEqual(restoreOptions(initial,'null','null'),initial);
});

test('rapid wall requests keep one active calculation and only the latest queued request',async()=>{
 const queue=new RegionSelectionQueue(),result=deferred<any>();let current=1;const calls:number[]=[];
 const bridge={invoke:async<T>(_command:string,args:Record<string,unknown>)=>{calls.push(args.faceId as number);return (calls.length===1?result.promise:Promise.resolve({request_id:7,face_ids:[1]})) as Promise<T>;}};
 const first=queue.request(bridge,document,0,()=>current===1);
 current=2;const replaced=queue.request(bridge,document,0,()=>current===2);
 current=3;const latest=queue.request(bridge,document,1,()=>current===3);
 assert.deepEqual(calls,[0]);assert.equal(await replaced,null);
 result.resolve({request_id:7,face_ids:[0]});assert.equal(await first,null);assert.deepEqual(await latest,[1]);assert.deepEqual(calls,[0,1]);
});
