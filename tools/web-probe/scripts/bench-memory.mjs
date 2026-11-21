// Fresh engine instance per pane count. Linear memory is retained capacity,
// not live allocation size, process RSS, GPU memory or total browser memory.
import {readFile} from 'node:fs/promises';
import {cpus} from 'node:os';
import assert from 'node:assert/strict';
import {loadNativeVt,createNativeHost} from '../native-vt.js';
const bytes = await readFile(new URL('../../../.deps/ghostty-vt-wasm32-freestanding/bin/ghostty-vt.wasm',import.meta.url));
console.log(JSON.stringify({kind:'environment',node:process.version,platform:process.platform,arch:process.arch,
 cpu:cpus()[0].model,scope:'engine WASM linear memory capacity; excludes UI, JS, GPU and browser overhead'}));
const output = Array.from({length:2000},(_,i)=>`Sample log ${i}: ${'x'.repeat(64)}\r\n`).join('');
for (const panes of [1,4,16]) {
 const host = createNativeHost(await loadNativeVt(bytes),()=>false);
 const baseline = host.memoryStats();
 for (let cycle=1;cycle<=3;cycle++) {
  const ids=Array.from({length:panes},()=>host.create(80,24,undefined,undefined,4*1024*1024));
  for (const id of ids) {host.write(id,output);host.snapshot(id);}
  const populated=host.memoryStats();
  assert.equal(populated.sessions,panes);
  ids.forEach(id=>host.free(id));
  const closed=host.memoryStats();
  assert.equal(closed.sessions,0);
  console.log(JSON.stringify({kind:'memory',panes,cycle,columns:80,rows:24,history_limit_bytes:4*1024*1024,
   output_bytes_per_pane:Buffer.byteLength(output),baseline,populated,closed}));
 }
}
