// Node engine/JSON boundary microbenchmark, excluding UI and Rust deserialization.
import {readFile} from 'node:fs/promises';
import {spawnSync} from 'node:child_process';
import {cpus} from 'node:os';
import assert from 'node:assert/strict';
import {Ghostty} from '../dist/ghostty-web.js';
import {createHost} from '../dist/engine.js';
const [countArg, scenario] = process.argv.slice(2);
if (!countArg) {
  console.log(JSON.stringify({kind:'environment',date:new Date().toISOString(),node:process.version,
    platform:process.platform,arch:process.arch,cpu:cpus()[0].model,ghosttyWeb:'0.4.0',
    scope:'Node Ghostty WASM write + sparse JSON serialization; excludes Rust, shaping, GPU and presentation'}));
  for (const panes of [1,4,16]) for (const mode of ['clean-snapshot','one-row','full-viewport']) {
    const result=spawnSync(process.execPath,['--expose-gc',import.meta.filename,String(panes),mode],{encoding:'utf8'});
    if(result.status!==0)throw new Error(result.stderr||result.error||`child exit ${result.status}`);
    process.stdout.write(result.stdout);
  }
} else {
  const panes=Number(countArg),columns=80,height=24,iterations=100,warmup=20;
  const wasm=await readFile(new URL('../dist/ghostty-vt.wasm',import.meta.url));
  const host=createHost(await Ghostty.load('data:application/wasm;base64,'+wasm.toString('base64')));
  const ids=Array.from({length:panes},()=>host.create(columns,height));
  for(const id of ids)host.snapshot(id);
  global.gc?.();
  const before=process.memoryUsage(),samples=[],payloads=[];
  let transferredRows=0;
  for(let i=0;i<warmup+iterations;i++) {
    const row=String(i%10).repeat(columns);
    const update=scenario==='clean-snapshot'?'':scenario==='one-row'?'\x1b[H'+row
      :'\x1b[H'+Array.from({length:height},()=>row).join('\r\n');
    const start=performance.now();
    const frames=ids.map(id=>{if(update)host.write(id,update);return host.snapshot(id);});
    const duration=performance.now()-start;
    const patches=frames.map(text=>JSON.parse(text));
    const expected=scenario==='clean-snapshot'?0:scenario==='one-row'?1:height;
    for(const patch of patches)assert.equal(patch.rows.length,expected,'dirty row contract');
    if(i>=warmup) {
      samples.push(duration);payloads.push(frames.reduce((n,text)=>n+Buffer.byteLength(text),0));
      transferredRows+=patches.reduce((n,patch)=>n+patch.rows.length,0);
    }
  }
  global.gc?.();
  const after=process.memoryUsage();samples.sort((a,b)=>a-b);
  console.log(JSON.stringify({kind:'result',panes,scenario,columns,height,iterations,warmup,
    median_ms:samples[Math.floor(iterations*.5)],p95_ms:samples[Math.ceil(iterations*.95)-1],
    mean_payload_bytes:payloads.reduce((a,b)=>a+b,0)/iterations,mean_transferred_rows:transferredRows/iterations,
    process_before_bytes:before,process_after_gc_bytes:after,
    memory_note:'whole isolated Node process; not per-pane/browser memory or transient allocation peak'}));
  ids.forEach(id=>host.free(id));
}
