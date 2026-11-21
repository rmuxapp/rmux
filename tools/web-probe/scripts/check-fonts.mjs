import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {readFont, FONT_ASSETS} from '../fonts.js';
for (const [kind, asset] of Object.entries(FONT_ASSETS)) {
  const bytes = await readFile(new URL(`../assets/${asset.file}`, import.meta.url));
  assert.equal((await readFont(new Response(bytes), kind)).length, asset.size);
  await assert.rejects(readFont(new Response('', {status:404}), kind), /unavailable/);
  await assert.rejects(readFont(new Response(new Uint8Array(3)), kind), /Incomplete/);
  await assert.rejects(readFont(new Response(new Uint8Array(asset.size + 1)), kind), /too large/);
  await assert.rejects(readFont(new Response(new Uint8Array(asset.size)), kind), /checksum/);
}
await assert.rejects(readFont(new Response(''), '__proto__'), /Unknown/);
console.log('Optional fonts: pinned CJK/emoji assets and bounded failure paths passed.');
