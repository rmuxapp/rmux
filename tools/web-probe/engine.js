// Browser VT host only: all layout and terminal drawing remain in Rust.
import {loadFont} from './fonts.js';
import {loadNativeVt,createNativeHost} from './native-vt.js';
export function isMacPlatform(platform = globalThis.navigator?.userAgentData?.platform ?? globalThis.navigator?.platform ?? '') {
  return /^(Mac|iPhone|iPad|iPod)/i.test(platform);
}
export async function initializeEngine() {
  globalThis.rmuxFonts = {loadFont};
  const response=await fetch(new URL('./native-ghostty-vt.wasm',import.meta.url));
  if (!response.ok) throw new Error(`Loading libghostty-vt failed: ${response.status}`);
  globalThis.rmuxEngine = createNativeHost(await loadNativeVt(await response.arrayBuffer()),isMacPlatform);
  // Browser menu/OS copy commands can arrive without a keydown. Populate the
  // trusted clipboard event synchronously, only while a terminal has UI focus.
  document.addEventListener('copy',event=>copyTerminalSelection(event,globalThis.rmuxEngine));
}
// A failed terminal copy must not fall through to copying the hidden input or
// unrelated document selection. Leave clipboard contents untouched on failure.
export function copyTerminalSelection(event,engine,report = message=>console.warn(message)) {
  try {
    const text=engine.copySelection();
    if (text===null || !event.clipboardData) return;
    event.clipboardData.setData('text/plain',text);
    event.preventDefault();
  } catch {
    event.preventDefault();
    report('Could not copy terminal selection. Maximum size is 1 MiB.');
  }
}
