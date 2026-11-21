// Direct adapter for the same pinned libghostty-vt C API used by rmux desktop.
import {createRenderAdapter} from './native-vt-render.js';
import {createSelectionAdapter} from './native-vt-selection.js';
export async function loadNativeVt(bytes) {
  const {instance} = await WebAssembly.instantiate(bytes, {});
  const api = instance.exports;
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
      let encoder = 0, event = 0, renderer, selection;
      const alive = () => { if (!terminal) throw new Error('Terminal disposed'); };
      return {
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
        selectedText(limit = 1024 * 1024, unwrap = true) {
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
                return buffer(4, written => buffer(limit, output => {
                  check(api.ghostty_formatter_format_buf(formatter, output, limit, written));
                  const length = new DataView(api.memory.buffer).getUint32(written, true);
                  return decoder.decode(memory().subarray(output, output + length));
                }));
              });
            } finally { api.ghostty_formatter_free(formatter); }
          });
        },
        dispose() {
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
    create(columns,rows,foreground,background,scrollback) {
      const terminal=engine.create(columns,rows,{foreground,background,scrollback});
      const id=next++; sessions.set(id,terminal); return id;
    },
    write(id,text) { sessions.get(id).write(text); },
    resize(id,columns,rows) { sessions.get(id).resize(columns,rows); },
    snapshot(id) { return JSON.stringify(sessions.get(id).snapshot()); },
    select(id,...args) { return sessions.get(id).select(...args); },
    selectedText(id) { return sessions.get(id).selectedText(); },
    scroll(id,kind,delta) { sessions.get(id).scroll(kind,delta); },
    focus(id,active) { if (active) focused=id; else if (focused===id) focused=null; },
    copySelection() { return focused===null ? null : sessions.get(focused)?.selectedText() ?? null; },
    encode(id,key,mods,repeat,text) {
      return new TextDecoder().decode(sessions.get(id).encode(key,{shift:!!(mods&1),control:!!(mods&2),alt:!!(mods&4),action:repeat?'REPEAT':'PRESS',text}));
    },
    free(id) { if (focused===id) focused=null; sessions.get(id)?.dispose(); sessions.delete(id); },
  };
}
