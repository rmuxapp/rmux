// Direct adapter for the same pinned libghostty-vt C API used by rmux desktop.
// Not yet wired into the browser renderer; engine migration is incremental.
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
    create(columns, rows) {
      let terminal = handle(slot => api.ghostty_terminal_new(0, slot, columns, rows));
      const alive = () => { if (!terminal) throw new Error('Terminal disposed'); };
      return {
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
        dispose() { if (terminal) { api.ghostty_terminal_free(terminal); terminal = 0; } },
      };
    },
  };
}
