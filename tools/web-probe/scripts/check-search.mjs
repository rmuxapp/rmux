import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {loadNativeVt} from '../native-vt.js';
const host = await loadNativeVt(await readFile(new URL('../../../.deps/ghostty-vt-wasm32-freestanding/bin/ghostty-vt.wasm',import.meta.url)));
const terminal = host.create(20,3);
function finish() {
  for(let n=0;n<1000;n++) { const progress=terminal.searchStep(); if(!progress.pending) return progress; }
  throw new Error('Search did not finish within bounded fixture steps');
}
try {
  assert.deepEqual(terminal.searchStep(),{matches:0,selected:null,pending:false});
  terminal.write('older needle\r\n日本語\r\nlast NEEDLE\r\n');
  terminal.searchQuery('needle');
  assert.equal(finish().matches,2);
  assert.equal(terminal.searchNavigate(true).selected,0);
  assert.equal(terminal.selectedText().toLowerCase(),'needle');
  assert.equal(terminal.searchNavigate(true).selected,1);
  assert.equal(terminal.searchNavigate(false).selected,0);
  terminal.searchQuery('日本語');
  assert.equal(finish().matches,1);
  terminal.searchNavigate(true);
  assert.equal(terminal.selectedText(),'日本語');
  terminal.searchQuery('absent');
  assert.equal(finish().matches,0);
  terminal.write('absent\r\n'); terminal.searchRefresh();
  assert.equal(finish().matches,1);
  assert.throws(()=>terminal.searchQuery('é'.repeat(2049)),RangeError);
  assert.equal(finish().matches,1); // Rejected query does not replace the live one.
  assert.throws(()=>terminal.searchQuery('\ud800'),RangeError);
  terminal.searchQuery(''); assert.equal(finish().matches,0);
  terminal.searchClose(); terminal.searchClose();
  terminal.searchQuery('needle'); assert.equal(finish().matches,2);
} finally { terminal.dispose(); terminal.dispose(); }
assert.throws(()=>terminal.searchStep(),/disposed/);
console.log('Incremental Ghostty search, navigation, Unicode, refresh and disposal passed');
