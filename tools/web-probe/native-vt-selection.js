// The native selection gesture API owns anchors, word boundaries and ranges.
export function createSelectionAdapter(api, manifest, terminal, {buffer, handle, check, value}) {
  const gesture=handle(slot=>api.ghostty_selection_gesture_new(0,slot));
  const events=new Map();
  const field=(type,name)=>manifest.types[type].fields[name].offset;
  return {
    reset() { api.ghostty_selection_gesture_reset(gesture,terminal); },
    dispose() {
      for (const event of events.values()) api.ghostty_selection_gesture_event_free(event);
      api.ghostty_selection_gesture_free(gesture);
    },
    select(action,x,y,cellWidth,cellHeight,columns,rows,rectangle=false,clicks=1) {
      if (!(cellWidth>0 && cellHeight>0 && columns>0 && rows>0) || !Number.isFinite(x+y+cellWidth+cellHeight)) throw new RangeError('Invalid selection geometry');
      const kind=value('GhosttySelectionGestureEventType',action);
      if (kind===undefined) throw new RangeError('Invalid selection action');
      if (!events.has(action)) events.set(action,handle(slot=>api.ghostty_selection_gesture_event_new(0,slot,kind)));
      const event=events.get(action);
      return buffer(160,p=>{
        new Uint8Array(api.memory.buffer,p,160).fill(0);
        const view=()=>new DataView(api.memory.buffer);
        const put=(at,n)=>view().setUint32(at,n,true);
        const set=(key,ptr)=>check(api.ghostty_selection_gesture_event_set(event,value('GhosttySelectionGestureEventOption',key),ptr));
        const coordinate=p+field('GhosttyPoint','value');
        put(p+field('GhosttyPoint','tag'),value('GhosttyPointTag','VIEWPORT'));
        view().setUint16(coordinate+field('GhosttyPointCoordinate','x'),Math.min(columns-1,Math.floor(Math.max(0,x)/cellWidth)),true);
        put(coordinate+field('GhosttyPointCoordinate','y'),Math.min(rows-1,Math.floor(Math.max(0,y)/cellHeight)));
        const grid=p+32, position=p+48, behaviors=p+64, geometry=p+80, selected=p+104;
        if (action==='AUTOSCROLL_TICK') set('VIEWPORT',coordinate);
        else { check(api.ghostty_terminal_grid_ref(terminal,p,grid)); set('REF',grid); }
        const scale=1024/cellWidth;
        if (action!=='RELEASE') {
          view().setFloat64(position+field('GhosttySurfacePosition','x'),x*scale,true);
          view().setFloat64(position+field('GhosttySurfacePosition','y'),y*scale,true);
          set('POSITION',position);
        }
        if (action==='PRESS') {
          const behavior=value('GhosttySelectionGestureBehavior',clicks>=3?'LINE':clicks===2?'WORD':'CELL');
          for (const name of ['single_click','double_click','triple_click']) put(behaviors+field('GhosttySelectionGestureBehaviors',name),behavior);
          set('BEHAVIORS',behaviors);
        }
        if (action==='DRAG' || action==='AUTOSCROLL_TICK') {
          for (const [name,n] of Object.entries({columns,cell_width:1024,padding_left:0,screen_height:Math.max(1,Math.round(rows*cellHeight*scale))})) put(geometry+field('GhosttySelectionGestureGeometry',name),n);
          set('GEOMETRY',geometry);
          view().setUint8(p+128,Number(rectangle)); set('RECTANGLE',p+128);
        }
        const result=api.ghostty_selection_gesture_event(gesture,terminal,event,selected);
        if (result===0) check(api.ghostty_terminal_set(terminal,value('GhosttyTerminalOption','SELECTION'),selected));
        else if (result===value('GhosttyResult','NO_VALUE')) {
          if (action==='PRESS') check(api.ghostty_terminal_set(terminal,value('GhosttyTerminalOption','SELECTION'),0));
        } else check(result);
      });
    },
  };
}
