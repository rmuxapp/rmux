// Unmodified, self-hosted Noto CJK; never part of the initial WASM download.
export const CJK_SIZE = 16467736;
export const CJK_SHA256 = '68a3fc98800b2a27b371f2fb79991daf3633bd89309d4ffaa6946fd587f375b5';
export async function readCjk(response) {
  if (!response.ok || !response.body) throw new Error('CJK font unavailable');
  const bytes = new Uint8Array(CJK_SIZE);
  const reader = response.body.getReader();
  let offset = 0;
  try {
    for (;;) {
      const {done, value} = await reader.read();
      if (done) break;
      if (offset + value.length > CJK_SIZE) throw new Error('CJK font too large');
      bytes.set(value, offset); offset += value.length;
    }
    if (offset !== CJK_SIZE) throw new Error('Incomplete CJK font');
    const digest = new Uint8Array(await crypto.subtle.digest('SHA-256', bytes));
    if (Array.from(digest, b => b.toString(16).padStart(2, '0')).join('') !== CJK_SHA256) throw new Error('CJK font checksum mismatch');
    return bytes;
  } finally { await reader.cancel(); reader.releaseLock(); }
}
export async function loadCjk() {
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), 30000);
  try { return await readCjk(await fetch(new URL('./fonts/NotoSansCJKjp-Regular.otf', import.meta.url), {signal:controller.signal})); }
  finally { clearTimeout(timeout); }
}
