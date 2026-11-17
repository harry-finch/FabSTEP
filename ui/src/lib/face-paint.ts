/** A stroke adds each visible face once. A short stationary gesture remains a click. */
export class FacePaintStroke {
  pointer:number|null=null;
  dragging=false;
  private start:[number,number]=[0,0];
  private sampled:[number,number]=[0,0];
  private latest:[number,number]=[0,0];
  private seen=new Set<number>();
  begin(pointer:number,x:number,y:number) {
    if(this.pointer!==null)return false;
    this.pointer=pointer;this.start=[x,y];this.sampled=[x,y];this.latest=[x,y];this.dragging=false;this.seen.clear();return true;
  }
  move(pointer:number,x:number,y:number) {
    if(pointer!==this.pointer)return false;
    this.latest=[x,y];
    if(Math.hypot(x-this.start[0],y-this.start[1])>4)this.dragging=true;
    return this.dragging;
  }
  sample(pick:(x:number,y:number)=>number|null):number[] {
    if(this.pointer===null||!this.dragging)return [];
    const [x,y]=this.sampled,[endX,endY]=this.latest;
    const n=Math.min(63,Math.max(1,Math.ceil(Math.hypot(endX-x,endY-y)/4)));
    const added:number[]=[];
    for(let i=0;i<=n;i++) {
      const face=pick(x+(endX-x)*i/n,y+(endY-y)*i/n);
      if(face!==null&&!this.seen.has(face)&&this.seen.size<100_000){this.seen.add(face);added.push(face);}
    }
    this.sampled=[endX,endY];return added;
  }
  finish(){const dragged=this.dragging;this.cancel();return dragged;}
  cancel(){this.pointer=null;this.dragging=false;this.seen.clear();}
}

/** One worker update at a time; pending additions are unioned, never toggled or lost. */
export class FacePaintQueue {
  private active=false;
  private pending=new Set<number>();
  private send:((ids:number[])=>Promise<void>)|null=null;
  add(ids:number[],send:(ids:number[])=>Promise<void>) {
    for(const id of ids)if(this.pending.size<100_000)this.pending.add(id);
    this.send=send;
    void this.flush();
  }
  clear(){this.pending.clear();this.send=null;}
  private async flush(){
    if(this.active||!this.pending.size||!this.send)return;
    const ids=[...this.pending],send=this.send;this.pending.clear();this.active=true;
    try{await send(ids);}catch{this.clear();}finally{this.active=false;void this.flush();}
  }
}
