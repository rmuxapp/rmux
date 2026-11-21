// Snapshot adapter for libghostty-vt's native render-state API.
export function createRenderAdapter(api, manifest, terminal, {buffer, handle, check, value}) {
  let render = 0, iterator = 0, cells = 0, revision = 0;
  const dispose = () => {
    if (cells) api.ghostty_render_state_row_cells_free(cells);
    if (iterator) api.ghostty_render_state_row_iterator_free(iterator);
    if (render) api.ghostty_render_state_free(render);
    cells = iterator = render = 0;
  };
  try {
    render = handle(slot => api.ghostty_render_state_new(0, slot));
    iterator = handle(slot => api.ghostty_render_state_row_iterator_new(0, slot));
    cells = handle(slot => api.ghostty_render_state_row_cells_new(0, slot));
  } catch (error) { dispose(); throw error; }
  const view = () => new DataView(api.memory.buffer);
  const offset = (type, field) => manifest.types[type].fields[field].offset;
  const u8 = p => view().getUint8(p), u16 = p => view().getUint16(p,true), u32 = p => view().getUint32(p,true);
  const put = (p, n) => view().setUint32(p,n,true);
  const rgb = p => [u8(p),u8(p+1),u8(p+2)];
  const decoder = new TextDecoder();
  return {
    dispose,
    snapshot() {
      check(api.ghostty_render_state_update(render, terminal));
      // One scratch arena per snapshot, never allocate in the cell loop.
      return buffer(2048, base => {
        const color = base, cursor = base+800, selection = base+832, style = base+848;
        const raw = base+864, glyph = base+880, text = base+896, slot = base+1920, ySlot = base+1924;
        const get = (key, ptr) => check(api.ghostty_render_state_get(render, value('GhosttyRenderStateData',key), ptr));
        const cellGet = (key, ptr) => check(api.ghostty_render_state_row_cells_get(cells, value('GhosttyRenderStateRowCellsData',key), ptr));
        put(color,manifest.types.GhosttyRenderStateColors.size); get('COLORS',color);
        put(cursor,manifest.types.GhosttyRenderStateCursor.size); get('CURSOR',cursor);
        get('COLS',slot); const columns = u16(slot);
        get('ROWS',slot); const height = u16(slot);
        const background = rgb(color+offset('GhosttyRenderStateColors','background'));
        const foreground = rgb(color+offset('GhosttyRenderStateColors','foreground'));
        const palette = color+offset('GhosttyRenderStateColors','palette');
        const resolve = (p, fallback) => {
          const tag=u8(p+offset('GhosttyStyleColor','tag')), data=p+offset('GhosttyStyleColor','value');
          if (tag === value('GhosttyStyleColorTag','RGB')) return rgb(data);
          if (tag === value('GhosttyStyleColorTag','PALETTE')) return rgb(palette+u8(data)*3);
          return fallback;
        };
        const rows=[], selections=[];
        put(slot,iterator); get('ROW_ITERATOR',slot);
        while (api.ghostty_render_state_row_iterator_next_dirty(iterator,ySlot)) {
          const y=u16(ySlot), row=[];
          put(selection,manifest.types.GhosttyRenderStateRowSelection.size);
          const selected=api.ghostty_render_state_row_get(iterator,value('GhosttyRenderStateRowData','SELECTION'),selection);
          if (selected !== value('GhosttyResult','NO_VALUE')) check(selected);
          selections.push([y,selected===0 ? [u16(selection+offset('GhosttyRenderStateRowSelection','start_x')),u16(selection+offset('GhosttyRenderStateRowSelection','end_x'))] : null]);
          put(slot,cells); check(api.ghostty_render_state_row_get(iterator,value('GhosttyRenderStateRowData','CELLS'),slot));
          while (api.ghostty_render_state_row_cells_next(cells)) {
            cellGet('RAW',raw); cellGet('STYLE',style);
            const wideField=manifest.types.GhosttyCell.bits.wide;
            const wide=Number((view().getBigUint64(raw,true)>>BigInt(wideField.lsb)) & ((1n<<BigInt(wideField.width))-1n));
            const width=wide===value('GhosttyCellWide','WIDE') ? 2 : wide===value('GhosttyCellWide','NARROW') ? 1 : 0;
            const setGlyph = (p,capacity) => {
              put(glyph+offset('GhosttyBuffer','ptr'),p); put(glyph+offset('GhosttyBuffer','cap'),capacity); put(glyph+offset('GhosttyBuffer','len'),0);
              return api.ghostty_render_state_row_cells_get(cells,value('GhosttyRenderStateRowCellsData','GRAPHEMES_UTF8'),glyph);
            };
            const decode = p => decoder.decode(new Uint8Array(api.memory.buffer,p,u32(glyph+offset('GhosttyBuffer','len'))));
            const result=setGlyph(text,1024);
            let label;
            if (result===value('GhosttyResult','OUT_OF_SPACE')) {
              const required=u32(glyph+offset('GhosttyBuffer','len'));
              label=buffer(required,p => { check(setGlyph(p,required)); return decode(p); });
            } else { check(result); label=decode(text); }
            const flags=u16(style+offset('GhosttyStyle','flags'));
            let fg=resolve(style+offset('GhosttyStyle','fg_color'),foreground);
            let bg=resolve(style+offset('GhosttyStyle','bg_color'),background);
            if (flags & 16) [fg,bg]=[bg,fg];
            let underline=resolve(style+offset('GhosttyStyle','underline_color'),fg);
            const dim=c => c.map((v,i)=>Math.floor((v+bg[i])/2));
            if (flags & 4) { fg=dim(fg); underline=dim(underline); }
            if (flags & 32) { fg=bg; underline=bg; }
            const wireFlags=(flags&3) | ((flags&0x700)?4:0) | ((flags&64)?8:0) | (flags&32);
            row.push([label,width,fg,bg,wireFlags,underline,((flags>>8)&7)===value('GhosttySgrUnderline','CURLY'),!!(flags&128)]);
          }
          rows.push([y,row]);
        }
        const cfield=name=>cursor+offset('GhosttyRenderStateCursor',name);
        const visible=u8(cfield('visible')) && u8(cfield('viewport_has_value'));
        const shape=u32(cfield('visual_style'));
        const explicit=!!u8(color+offset('GhosttyRenderStateColors','cursor_has_value'));
        const scrollbar=base+1952;
        check(api.ghostty_terminal_get(terminal,value('GhosttyTerminalData','SCROLLBAR'),scrollbar));
        const scroll=name=>Number(view().getBigUint64(scrollbar+offset('GhosttyTerminalScrollbar',name),true));
        const scrollback_lines=Math.max(0,scroll('total')-scroll('len'));
        const patch={revision:++revision,columns,height,rows,selections,background,
          cursor:visible ? [Math.max(0,u16(cfield('viewport_x'))-u8(cfield('wide_tail'))),u16(cfield('viewport_y'))] : null,
          cursor_shape:shape===value('GhosttyCursorStyle','BAR') ? 'Bar' : shape===value('GhosttyCursorStyle','UNDERLINE') ? 'Underline' : shape===value('GhosttyCursorStyle','BLOCK_HOLLOW') ? 'Hollow' : 'Block',
          cursor_color:explicit ? rgb(color+offset('GhosttyRenderStateColors','cursor')) : [140,180,255],
          cursor_color_explicit:explicit,scrollback_lines,scrollback_offset:Math.max(0,scrollback_lines-scroll('offset'))};
        check(api.ghostty_render_state_clean(render));
        return patch;
      });
    },
  };
}
