// Artifact sizes describe transfer payloads, not runtime memory or latency.
import {readFile, writeFile} from 'node:fs/promises';
import {gzipSync, brotliCompressSync} from 'node:zlib';
import {createHash} from 'node:crypto';
const files = ['rmux_web_probe_bg.wasm','native-ghostty-vt.wasm'];
const modules = [];
for (const file of files) {
  const bytes = await readFile(new URL(`../dist-release/${file}`, import.meta.url));
  modules.push({file, bytes:bytes.length, gzipBytes:gzipSync(bytes).length,
    brotliBytes:brotliCompressSync(bytes).length,
    sha256:createHash('sha256').update(bytes).digest('hex')});
}
const report = {scope:'WASM payloads only; excludes JS, fonts embedded in UI WASM are included; not runtime memory', modules};
await writeFile(new URL('../dist-release/build-sizes.json',import.meta.url),JSON.stringify(report,null,2)+'\n');
console.log(JSON.stringify(report,null,2));
