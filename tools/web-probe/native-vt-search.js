// Incremental search through the same pinned Ghostty API as desktop.
export function createSearchAdapter(api, manifest, terminal, {buffer, handle, check, value}) {
  let search = 0;
  const read = name => buffer(4, ptr => {
    const result = api.ghostty_search_get(search, value('GhosttySearchData', name), ptr);
    if (result === value('GhosttyResult', 'NO_VALUE')) return null;
    check(result);
    return new DataView(api.memory.buffer).getUint32(ptr, true);
  });
  const progress = () => search ? {
    matches: read('TOTAL_MATCHES'), selected: read('SELECTED_INDEX'),
    pending: read('STATUS') !== value('GhosttySearchStatus', 'COMPLETE'),
  } : {matches: 0, selected: null, pending: false};
  return {
    query(text) {
      if (!text.isWellFormed() || text.length > 4096) throw new RangeError('Invalid search query');
      const bytes = new TextEncoder().encode(text);
      if (bytes.length > 4096) throw new RangeError('Search query exceeds 4096 bytes');
      if (!search && !bytes.length) return progress();
      search ||= handle(slot => api.ghostty_search_new(0, slot, terminal));
      const type = manifest.types.GhosttyString;
      buffer(Math.max(1, bytes.length), ptr => buffer(type.size, string => {
        new Uint8Array(api.memory.buffer).set(bytes, ptr);
        const view = new DataView(api.memory.buffer);
        view.setUint32(string + type.fields.ptr.offset, ptr, true);
        view.setUint32(string + type.fields.len.offset, bytes.length, true);
        check(api.ghostty_search_set(search, value('GhosttySearchOption', 'NEEDLE'), string));
      }));
      return progress();
    },
    refresh() { if (search) check(api.ghostty_search_feed(search)); return progress(); },
    step() {
      if (!search) return progress();
      let status = read('STATUS');
      if (status === value('GhosttySearchStatus', 'FEED_REQUIRED')) {
        check(api.ghostty_search_feed(search));
        status = read('STATUS');
      }
      if (status === value('GhosttySearchStatus', 'RUNNING')) {
        buffer(4, ptr => check(api.ghostty_search_tick(search, ptr)));
      }
      return progress(); // One feed/tick maximum; caller yields between steps.
    },
    navigate(older) {
      if (!search || !read('TOTAL_MATCHES')) return progress();
      const result = older ? api.ghostty_search_select_next(search) : api.ghostty_search_select_prev(search);
      if (result !== value('GhosttyResult', 'NO_VALUE')) {
        check(result);
        buffer(manifest.types.GhosttySelection.size, ptr => {
          check(api.ghostty_search_get(search, value('GhosttySearchData', 'SELECTED_MATCH'), ptr));
          check(api.ghostty_terminal_set(terminal, value('GhosttyTerminalOption', 'SELECTION'), ptr));
        });
      }
      return progress();
    },
    dispose() { if (search) { api.ghostty_search_free(search); search = 0; } },
  };
}
