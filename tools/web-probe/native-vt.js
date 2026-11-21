import {createSearchAdapter} from './native-vt-search.js';
// Direct adapter for the same pinned libghostty-vt C API used by rmux desktop.
import {createRenderAdapter} from './native-vt-render.js';
import {createSelectionAdapter} from './native-vt-selection.js';
export async function loadNativeVt(bytes) {
  const {instance} = await WebAssembly.instantiate(bytes, {});
  const api = instance.exports;
  // A typed WASM import/export trampoline lets Ghostty call a JS writer through
  // its C function table. One slot per engine instance, reused synchronously.
  let writerSlot, writeChunk;
  function streamPrefix(formatter, limit) {
    if (writerSlot === undefined) {
      const trampoline = new WebAssembly.Instance(new WebAssembly.Module(new Uint8Array([
        0,97,115,109,1,0,0,0,
        1,8,1,96,3,127,127,127,1,127, // (i32,i32,i32) -> i32
        2,7,1,1,101,1,119,0,0, // import e.w
        7,5,1,1,119,0,0, // export w
      ])), {e:{w:(_user,ptr,len)=>writeChunk(ptr,len)}});
      writerSlot = api.__indirect_function_table.grow(1);
      api.__indirect_function_table.set(writerSlot,trampoline.exports.w);
    }
    if (writeChunk) throw new Error('Reentrant selection formatter');
    const output = new Uint8Array(limit);
    let length = 0, truncated = false;
    writeChunk = (ptr,len) => {
      if (len > limit-length) { truncated = true; return 0; }
      output.set(memory().subarray(ptr,ptr+len),length); length += len; return 1;
    };
    try {
      return buffer(manifest.types.GhosttyWriter.size, writer => {
        const view = new DataView(api.memory.buffer);
        view.setUint32(writer+manifest.types.GhosttyWriter.fields.write.offset,writerSlot,true);
        view.setUint32(writer+manifest.types.GhosttyWriter.fields.userdata.offset,0,true);
        const result = api.ghostty_formatter_format(formatter,writer);
        if (truncated && result === value('GhosttyResult','IO_ERROR')) return null;
        check(result);
        return decoder.decode(output.subarray(0,length));
      });
    } finally { writeChunk = undefined; }
  }
  const decoder = new TextDecoder();
  const memory = () => new Uint8Array(api.memory.buffer);
  const manifestPtr = api.ghostty_type_json();
  const manifest = JSON.parse(decoder.decode(memory().subarray(manifestPtr, memory().indexOf(0, manifestPtr))));
  if (manifest.library_version !== '1.0.1' || manifest.abi.pointer_size !== 4 || manifest.abi.usize_size !== 4 || manifest.abi.endian !== 'little') {
    throw new Error('Unsupported libghostty-vt version or WASM ABI');
  }
  const value = (type, key) => manifest.types[type].values[key];
  const check = result => { if (result !== 0) throw new Error(`libghostty-vt error ${result}`); };
  function buffer(size, use) {
    const ptr = api.ghostty_wasm_alloc(size);
    if (!ptr) throw new Error('libghostty-vt allocation failed');
    try { return use(ptr); } finally { api.ghostty_wasm_free(ptr, size); }
  }
  function handle(create) {
    const slot = api.ghostty_wasm_alloc_opaque();
    if (!slot) throw new Error('libghostty-vt handle allocation failed');
    try { check(create(slot)); return api.ghostty_wasm_take_opaque(slot); }
    finally { api.ghostty_wasm_free_opaque(slot); }
  }
  return {
    manifest,
    graphemes(text) {
      if (!text.isWellFormed()) throw new RangeError('Invalid Unicode text');
      if (text.length > 1024*1024) throw new RangeError('Text exceeds measurement limit');
      const chars = Array.from(text), utf8 = new TextEncoder();
      if (utf8.encode(text).length > 1024*1024 || /[\u0000-\u001f\u007f-\u009f]/u.test(text)) throw new RangeError('Invalid terminal text');
      if (!chars.length) return [];
      return buffer(chars.length*4+1, ptr => {
        const view = new DataView(api.memory.buffer);
        chars.forEach((ch,i)=>view.setUint32(ptr+i*4,ch.codePointAt(0),true));
        const result=[];
        let offset=0,end=0;
        while (offset<chars.length) {
          const count=api.ghostty_unicode_grapheme_width(ptr+offset*4,chars.length-offset,ptr+chars.length*4);
          const width=memory()[ptr+chars.length*4];
          if (!count || count>chars.length-offset || width>2) throw new Error('Invalid grapheme measurement');
          for (let i=offset;i<offset+count;i++) {
            const cp=chars[i].codePointAt(0);
            end+=cp<=0x7f ? 1 : cp<=0x7ff ? 2 : cp<=0xffff ? 3 : 4;
          }
          offset+=count;result.push({end,width});
        }
        return result;
      });
    },
    memoryBytes() { return api.memory.buffer.byteLength; },
    create(columns, rows, options = {}) {
      let terminal = handle(slot => api.ghostty_terminal_new(0, slot, columns, rows));
      try {
        for (const [name, color] of [['COLOR_FOREGROUND', options.foreground], ['COLOR_BACKGROUND', options.background]]) {
          if (color===undefined) continue;
          if (!Number.isInteger(color) || color<0 || color>0xffffff) throw new RangeError('Invalid terminal color');
          buffer(manifest.types.GhosttyColorRgb.size,p=>{
            const bytes=new Uint8Array(api.memory.buffer);
            for (const [channel,shift] of [['r',16],['g',8],['b',0]]) bytes[p+manifest.types.GhosttyColorRgb.fields[channel].offset]=(color>>>shift)&255;
            check(api.ghostty_terminal_set(terminal,value('GhosttyTerminalOption',name),p));
          });
        }
        if (options.scrollback!==undefined) {
          if (!Number.isInteger(options.scrollback) || options.scrollback<0 || options.scrollback>0xffffffff) throw new RangeError('Invalid scrollback limit');
          buffer(4,p=>{
            new DataView(api.memory.buffer).setUint32(p,options.scrollback,true);
            check(api.ghostty_terminal_set(terminal,value('GhosttyTerminalOption','SCROLLBACK_MAX_BYTES'),p));
          });
        }
      } catch (error) { api.ghostty_terminal_free(terminal); throw error; }
      let encoder = 0, event = 0, renderer, selection, search;
      const alive = () => { if (!terminal) throw new Error('Terminal disposed'); };
      return {
        searchQuery(text) {
          alive();
          search ??= createSearchAdapter(api,manifest,terminal,{buffer,handle,check,value});
          return search.query(text);
        },
        searchStep() { alive(); return search?.step() ?? {matches:0,selected:null,pending:false}; },
        searchRefresh() { alive(); return search?.refresh() ?? {matches:0,selected:null,pending:false}; },
        searchNavigate(older) { alive(); return search?.navigate(older) ?? {matches:0,selected:null,pending:false}; },
        searchClose() { alive(); search?.dispose(); search = undefined; },
        scroll(kind,delta=0) {
          alive();
          const tag=value('GhosttyTerminalScrollViewportTag',kind);
          if (tag===undefined || !Number.isSafeInteger(delta) || delta < -2147483648 || delta > 2147483647) throw new RangeError('Invalid scroll');
          const type=manifest.types.GhosttyTerminalScrollViewport;
          buffer(type.size,p=>{
            new Uint8Array(api.memory.buffer,p,type.size).fill(0);
            const view=new DataView(api.memory.buffer);
            view.setUint32(p+type.fields.tag.offset,tag,true);
            view.setInt32(p+type.fields.value.offset,delta,true);
            check(api.ghostty_terminal_scroll_viewport(terminal,p));
          });
        },
        select(...args) {
          alive();
          selection ??= createSelectionAdapter(api,manifest,terminal,{buffer,handle,check,value});
          return selection.select(...args);
        },
        snapshot() {
          alive();
          renderer ??= createRenderAdapter(api,manifest,terminal,{buffer,handle,check,value});
          return renderer.snapshot();
        },
        resize(columns, rows) {
          alive();
          check(api.ghostty_terminal_resize(terminal,columns,rows,0,0));
        },
        encodePaste(text) {
          alive();
          if (text.length > 1024 * 1024) throw new RangeError('Paste exceeds 1 MiB input limit');
          const utf8 = new TextEncoder().encode(text);
          if (utf8.length > 1024 * 1024) throw new RangeError('Paste exceeds 1 MiB input limit');
          const type = manifest.types.GhosttyTerminalModeConfig;
          const bracketed = buffer(type.size, ptr => {
            new Uint8Array(api.memory.buffer, ptr, type.size).fill(0);
            new DataView(api.memory.buffer).setUint16(ptr + type.fields.mode.offset, 2004, true);
            check(api.ghostty_terminal_get(terminal, value('GhosttyTerminalData', 'MODE'), ptr));
            return !!memory()[ptr + type.fields.value.offset];
          });
          return buffer(Math.max(1, utf8.length), input => {
            memory().set(utf8, input);
            return buffer(utf8.length + 12, output => buffer(4, written => {
              check(api.ghostty_paste_encode(input, utf8.length, bracketed, output, utf8.length + 12, written));
              const length = new DataView(api.memory.buffer).getUint32(written, true);
              if (length > utf8.length + 12) throw new Error('Invalid encoded paste length');
              return memory().slice(output, output + length);
            }));
          });
        },
        encode(key, {shift = false, control = false, alt = false, action = 'PRESS', text = ''} = {}) {
          alive();
          const keyCode = value('GhosttyKey', key), actionCode = value('GhosttyKeyAction', action);
          if (keyCode === undefined || actionCode === undefined) throw new RangeError('Unknown key or action');
          const utf8 = new TextEncoder().encode(text);
          if (utf8.length > 4096) throw new RangeError('Key text too long');
          if (!encoder) encoder = handle(slot => api.ghostty_key_encoder_new(0, slot));
          if (!event) event = handle(slot => api.ghostty_key_event_new(0, slot));
          // Synchronize all negotiated modes, including Kitty press/release flags.
          check(api.ghostty_key_encoder_set_from_terminal(encoder, terminal));
          return buffer(4, written => {
            new DataView(api.memory.buffer).setUint32(written, value('GhosttyOptionAsAlt', 'TRUE'), true);
            check(api.ghostty_key_encoder_set(encoder, value('GhosttyKeyEncoderOption', 'MACOS_OPTION_AS_ALT'), written));
            check(api.ghostty_key_event_set_action(event, actionCode));
            check(api.ghostty_key_event_set_key(event, keyCode));
            // Modifier bits are fixed macros in the pinned key/event.h.
            check(api.ghostty_key_event_set_mods(event, Number(shift) | Number(control)<<1 | Number(alt)<<2));
            const codepoint = text.codePointAt(0) ?? 0;
            check(api.ghostty_key_event_set_unshifted_codepoint(event, codepoint >= 65 && codepoint <= 90 ? codepoint + 32 : codepoint));
            const string = manifest.types.GhosttyString;
            return buffer(string.size + utf8.length, ptr => {
              const data = ptr + string.size;
              memory().set(utf8, data);
              const view = new DataView(api.memory.buffer);
              view.setUint32(ptr + string.fields.ptr.offset, data, true);
              view.setUint32(ptr + string.fields.len.offset, utf8.length, true);
              // WASM lowers a by-value C struct parameter to its address.
              check(api.ghostty_key_event_set_utf8(event, ptr));
              const encode = size => buffer(size, output => {
                const result = api.ghostty_key_encoder_encode(encoder, event, output, size, written);
                const length = new DataView(api.memory.buffer).getUint32(written, true);
                if (result === value('GhosttyResult', 'OUT_OF_SPACE')) {
                  if (length <= size || length > 65536) throw new RangeError('Encoded key exceeds limit');
                  return encode(length);
                }
                check(result);
                return memory().slice(output, output + length);
              });
              return encode(128);
            });
          });
        },
        write(text) {
          alive();
          const encoded = new TextEncoder().encode(text);
          if (!encoded.length) return;
          buffer(encoded.length, ptr => {
            memory().set(encoded, ptr);
            api.ghostty_terminal_vt_write(terminal, ptr, encoded.length);
          });
        },
        selectAll() {
          alive();
          selection?.reset();
          buffer(manifest.types.GhosttySelection.size, selection => {
            check(api.ghostty_terminal_select_all(terminal, selection));
            check(api.ghostty_terminal_set(terminal, value('GhosttyTerminalOption','SELECTION'), selection));
          });
        },
        clearSelection() {
          alive();
          check(api.ghostty_terminal_set(terminal, value('GhosttyTerminalOption','SELECTION'), 0));
        },
        selectedSearchQuery() {
          const text = this.selectedText(4096, true, true);
          return text && !/[\u0000-\u001f\u007f-\u009f]/u.test(text) ? text : null;
        },
        selectedText(limit = 1024 * 1024, unwrap = true, overflowIsEmpty = false) {
          alive();
          if (!Number.isSafeInteger(limit) || limit < 1 || limit > 16 * 1024 * 1024) throw new RangeError('Invalid copy limit');
          return buffer(manifest.types.GhosttySelection.size, selection => {
            const result = api.ghostty_terminal_get(terminal, value('GhosttyTerminalData','SELECTION'), selection);
            if (result === value('GhosttyResult','NO_VALUE')) return null;
            check(result);
            const formatter = handle(slot => api.ghostty_formatter_terminal_new(0, slot, terminal));
            try {
              check(api.ghostty_formatter_set(formatter, value('GhosttyFormatterOption','SELECTION'), selection));
              return buffer(1, flag => {
                memory()[flag] = Number(unwrap);
                check(api.ghostty_formatter_set(formatter, value('GhosttyFormatterOption','UNWRAP'), flag));
                const text = streamPrefix(formatter,limit);
                if (text === null && !overflowIsEmpty) check(value('GhosttyResult','OUT_OF_SPACE'));
                return text;
              });
            } finally { api.ghostty_formatter_free(formatter); }
          });
        },
        dispose() {
          search?.dispose(); search = undefined;
          selection?.dispose(); selection = undefined;
          renderer?.dispose(); renderer = undefined;
          if (event) { api.ghostty_key_event_free(event); event = 0; }
          if (encoder) { api.ghostty_key_encoder_free(encoder); encoder = 0; }
          if (terminal) { api.ghostty_terminal_free(terminal); terminal = 0; }
        },
      };
    },
  };
}

export function createNativeHost(engine, isMac) {
  const sessions = new Map();
  let next = 0, focused = null;
  return {
    isMac,
    graphemes(text) { return JSON.stringify(engine.graphemes(text)); },
    memoryStats() { return {engine_linear_memory_bytes: engine.memoryBytes(), sessions: sessions.size}; },
    create(columns,rows,foreground,background,scrollback) {
      const terminal=engine.create(columns,rows,{foreground,background,scrollback});
      const id=next++; sessions.set(id,terminal); return id;
    },
    write(id,text) { sessions.get(id).write(text); },
    resize(id,columns,rows) { sessions.get(id).resize(columns,rows); },
    snapshot(id) { return JSON.stringify(sessions.get(id).snapshot()); },
    searchQuery(id,text) { return JSON.stringify(sessions.get(id).searchQuery(text)); },
    searchStep(id) { return JSON.stringify(sessions.get(id).searchStep()); },
    searchRefresh(id) { return JSON.stringify(sessions.get(id).searchRefresh()); },
    searchNavigate(id,older) { return JSON.stringify(sessions.get(id).searchNavigate(older)); },
    searchClose(id) { sessions.get(id).searchClose(); },
    select(id,...args) { return sessions.get(id).select(...args); },
    selectedText(id) { return sessions.get(id).selectedText(); },
    selectedSearchQuery(id) { return sessions.get(id).selectedSearchQuery(); },
    scroll(id,kind,delta) { sessions.get(id).scroll(kind,delta); },
    focus(id,active) { if (active) focused=id; else if (focused===id) focused=null; },
    copySelection() { return focused===null ? null : sessions.get(focused)?.selectedText() ?? null; },
    encodePaste(id,text) { return new TextDecoder().decode(sessions.get(id).encodePaste(text)); },
    encode(id,key,mods,action,text) {
      return new TextDecoder().decode(sessions.get(id).encode(key,{shift:!!(mods&1),control:!!(mods&2),alt:!!(mods&4),action,text}));
    },
    free(id) { if (focused===id) focused=null; sessions.get(id)?.dispose(); sessions.delete(id); },
  };
}
