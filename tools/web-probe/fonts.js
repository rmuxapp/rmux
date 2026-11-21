// Unmodified, self-hosted optional fonts; never part of the initial WASM download.
export const FONT_ASSETS = {
  cjk: {file:'NotoSansCJKjp-Regular.otf', size:16467736, sha256:'68a3fc98800b2a27b371f2fb79991daf3633bd89309d4ffaa6946fd587f375b5'},
  emoji: {file:'NotoEmoji.ttf', size:1982596, sha256:'de6c18832938afc99caf132b39d6a30a19bac7f2e812e28db2535b4608d27551'},
};
export async function readFont(response, kind) {
  const asset = Object.hasOwn(FONT_ASSETS, kind) && FONT_ASSETS[kind];
  if (!asset) throw new Error('Unknown optional font');
  if (!response.ok || !response.body) throw new Error('Optional font unavailable');
  const bytes = new Uint8Array(asset.size);
  const reader = response.body.getReader();
  let offset = 0;
  try {
    for (;;) {
      const {done, value} = await reader.read();
      if (done) break;
      if (offset + value.length > asset.size) throw new Error('Optional font too large');
      bytes.set(value, offset); offset += value.length;
    }
    if (offset !== asset.size) throw new Error('Incomplete Optional font');
    const digest = new Uint8Array(await crypto.subtle.digest('SHA-256', bytes));
    if (Array.from(digest, b => b.toString(16).padStart(2, '0')).join('') !== asset.sha256) throw new Error('Optional font checksum mismatch');
    return bytes;
  } finally { await reader.cancel(); reader.releaseLock(); }
}
export async function loadFont(kind) {
  const asset = Object.hasOwn(FONT_ASSETS, kind) && FONT_ASSETS[kind];
  if (!asset) throw new Error('Unknown optional font');
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), 30000);
  try { return await readFont(await fetch(new URL(`./fonts/${asset.file}`, import.meta.url), {signal:controller.signal}), kind); }
  finally { clearTimeout(timeout); }
}
