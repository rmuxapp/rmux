// Browser VT host only: all layout and terminal drawing remain in Rust.
import {Ghostty, CellFlags as F, KeyEncoderOption as O} from './ghostty-web.js';
export function createHost(engine) {
  const sessions = new Map();
  let next = 0;
  const rgb = c => [c.r, c.g, c.b];
  return {
    create(columns, rows) {
      const id = next++;
      const terminal = engine.createTerminal(columns, rows);
      terminal.write('\x1bc\x1b[2J\x1b[3J\x1b[H');
      sessions.set(id, {terminal, encoder:engine.createKeyEncoder(), revision:0, first:true});
      return id;
    },
    write(id, text) { sessions.get(id).terminal.write(text); },
    resize(id, columns, rows) {
      const state = sessions.get(id);
      if (state.terminal.cols === columns && state.terminal.rows === rows) return;
      state.terminal.resize(columns, rows);
      state.first = true;
    },
    free(id) { const s=sessions.get(id); s?.encoder.dispose(); s?.terminal.free(); sessions.delete(id); },
    encode(id, key, mods, repeat, text) {
      const {terminal,encoder}=sessions.get(id);
      encoder.setOption(O.CURSOR_KEY_APPLICATION,terminal.getMode(1));
      encoder.setOption(O.KEYPAD_KEY_APPLICATION,terminal.getMode(66));
      encoder.setOption(O.ALT_ESC_PREFIX,true);
      return new TextDecoder().decode(encoder.encode({key,mods,action:repeat?2:1,utf8:text}));
    },
    snapshot(id) {
      const state = sessions.get(id), t = state.terminal;
      t.update();
      const cursor = t.getCursor(), colors = t.getColors(), viewport = t.getViewport();
      const rows = [];
      for (let y = 0; y < t.rows; y++) {
        if (!state.first && !t.isRowDirty(y)) continue;
        const cells = [];
        for (let x = 0; x < t.cols; x++) {
          const c = viewport[y*t.cols+x];
          let foreground = [c.fg_r,c.fg_g,c.fg_b], background = [c.bg_r,c.bg_g,c.bg_b];
          if (c.flags & F.INVERSE) [foreground,background] = [background,foreground];
          if (c.flags & F.FAINT) foreground = foreground.map((v,i)=>Math.round((v+background[i])/2));
          // Wire tuple: text, width, resolved foreground/background, style flags.
          // Column is implied by row position; defaults are reconstructed in Rust.
          cells.push([c.grapheme_len ? t.getGraphemeString(y,x) : String.fromCodePoint(c.codepoint||32),
            c.width,foreground,background,c.flags]);
        }
        rows.push([y,cells]);
      }
      state.first = false;
      t.markClean();
      return JSON.stringify({revision:++state.revision,columns:t.cols,height:t.rows,rows,
        background:rgb(colors.background),cursor:cursor.visible ? [cursor.x,cursor.y] : null,
        cursor_shape:{block:'Block',bar:'Bar',underline:'Underline'}[cursor.style],
        cursor_color:rgb(colors.cursor||colors.foreground),cursor_color_explicit:!!colors.cursor,
        scrollback_lines:t.getScrollbackLength()});
    }
  };
}
export async function initializeEngine() {
  globalThis.rmuxEngine = createHost(await Ghostty.load(new URL('./ghostty-vt.wasm',import.meta.url).href));
}
