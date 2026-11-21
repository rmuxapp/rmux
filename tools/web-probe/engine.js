// Browser VT host only: all layout and terminal drawing remain in Rust.
import {loadNativeVt,createNativeHost} from './native-vt.js';
export function isMacPlatform(platform = globalThis.navigator?.userAgentData?.platform ?? globalThis.navigator?.platform ?? '') {
  return /^(Mac|iPhone|iPad|iPod)/i.test(platform);
}
export async function initializeEngine() {
  const response=await fetch(new URL('./native-ghostty-vt.wasm',import.meta.url));
  if (!response.ok) throw new Error(`Loading libghostty-vt failed: ${response.status}`);
  globalThis.rmuxEngine = createNativeHost(await loadNativeVt(await response.arrayBuffer()),isMacPlatform);
}
