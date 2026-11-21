// Fresh engine instance per pane count. Linear memory is retained capacity,
// not live allocation size, process RSS, GPU memory or total browser memory.
import {readFile} from 'node:fs/promises';
import {cpus} from 'node:os';
import {createHash} from 'node:crypto';
import {execFileSync} from 'node:child_process';
import assert from 'node:assert/strict';
import {loadNativeVt,createNativeHost} from '../native-vt.js';
const bytes = await readFile(new URL('../../../.deps/ghostty-vt-wasm32-freestanding/bin/ghostty-vt.wasm',import.meta.url));
console.log(JSON.stringify({kind:'environment',node:process.version,platform:process.platform,arch:process.arch,
 cpu:cpus()[0].model,source:execFileSync('git',['rev-parse','HEAD'],{encoding:'utf8'}).trim(),
 wasm_sha256:createHash('sha256').update(bytes).digest('hex'),scope:'engine WASM linear memory capacity; excludes UI, JS, GPU and browser overhead'}));
const output = Array.from({length:2000},(_,i)=>`Sample log ${i}: ${'x'.repeat(64)}\r\n`).join('');
for (const workload of ['output','search-copy']) for (const panes of [1,4,16]) {
 const host = createNativeHost(await loadNativeVt(bytes),()=>false);
 const baseline = host.memoryStats();
 for (let cycle=1;cycle<=5;cycle++) {
  const ids=Array.from({length:panes},()=>host.create(80,24,undefined,undefined,4*1024*1024));
  for (const id of ids) {host.write(id,output);host.snapshot(id);}
  let searchSteps=0;
  if(workload==='search-copy') for(const id of ids) {
   let progress=JSON.parse(host.searchQuery(id,'Sample'));
   for(let step=0;progress.pending && step<10000;step++) {
    progress=JSON.parse(host.searchStep(id)); searchSteps++;
   }
   assert.equal(progress.pending,false,'bounded search must finish');
   assert.ok(progress.matches>0);
   host.searchNavigate(id,true);
   assert.equal(host.selectedText(id),'Sample');
   assert.equal(host.selectedSearchQuery(id),'Sample');
   // Alternate explicit search closure and terminal destruction with a live
   // search handle; both paths must release their Ghostty-owned allocations.
   if(cycle%2===0) host.searchClose(id);
  }
  const populated=host.memoryStats();
  assert.equal(populated.sessions,panes);
  ids.forEach(id=>host.free(id));
  const closed=host.memoryStats();
  assert.equal(closed.sessions,0);
  console.log(JSON.stringify({kind:'memory',workload,panes,cycle,search_steps:searchSteps,columns:80,rows:24,history_limit_bytes:4*1024*1024,
   output_bytes_per_pane:Buffer.byteLength(output),baseline,populated,closed}));
 }
}
