import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {loadNativeVt} from '../native-vt.js';
const bytes = await readFile(new URL('../../../.deps/ghostty-vt-wasm32-freestanding/bin/ghostty-vt.wasm', import.meta.url));
const host = await loadNativeVt(bytes);
const first = host.create(5,4), second = host.create(20,4);
try {
  first.write('abcdefghij\r\nnext');
  assert.equal(first.selectedText(), null);
  first.selectAll();
  assert.equal(first.selectedText(), 'abcdefghij\nnext');
  assert.equal(first.selectedText(100, false), 'abcde\nfghij\nnext');
  assert.throws(() => first.selectedText(3));
  second.write('hello 世界');
  second.selectAll();
  assert.equal(second.selectedText(), 'hello 世界');
  first.clearSelection();
  assert.equal(first.selectedText(), null);
  assert.equal(second.selectedText(), 'hello 世界');
} finally { first.dispose(); second.dispose(); }
first.dispose();
assert.throws(() => first.write('after disposal'));
console.log('Pinned native VT WASM: selection, Unicode copy, soft wraps, limits and isolation passed.');
