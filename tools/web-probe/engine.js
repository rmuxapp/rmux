// Browser VT host only: all layout and terminal drawing remain in Rust.
import {loadFont} from './fonts.js';
import {loadNativeVt,createNativeHost} from './native-vt.js';
export function isMacPlatform(platform = globalThis.navigator?.userAgentData?.platform ?? globalThis.navigator?.platform ?? '') {
  return /^(Mac|iPhone|iPad|iPod)/i.test(platform);
}
export async function initializeEngine() {
  globalThis.rmuxFonts = {loadFont};
  globalThis.rmuxHelp = {open: openHelpLink};
  globalThis.rmuxClipboard = {readText: readTerminalClipboard};
  const response=await fetch(new URL('./native-ghostty-vt.wasm',import.meta.url));
  if (!response.ok) throw new Error(`Loading libghostty-vt failed: ${response.status}`);
  globalThis.rmuxEngine = createNativeHost(await loadNativeVt(await response.arrayBuffer()),isMacPlatform);
  // Browser menu/OS copy commands can arrive without a keydown. Populate the
  // trusted clipboard event synchronously, only while a terminal has UI focus.
  document.addEventListener('copy',event=>copyTerminalSelection(event,globalThis.rmuxEngine));
}
// Invoke the browser API synchronously under user activation. Never request
// image/HTML clipboard payloads for a text terminal or copy oversized text into WASM.
export function readTerminalClipboard(maxBytes, clipboard = globalThis.navigator?.clipboard) {
  if (!Number.isInteger(maxBytes) || maxBytes < 1 || maxBytes > 1024 * 1024) {
    return Promise.reject('Invalid clipboard limit.');
  }
  if (!clipboard?.readText) return Promise.reject('Clipboard access was unavailable. Use your keyboard paste shortcut.');
  let read;
  try { read = clipboard.readText(); }
  catch { return Promise.reject('Clipboard access was unavailable. Use your keyboard paste shortcut.'); }
  return Promise.resolve(read).then(text => {
    if (typeof text !== 'string') throw 'The clipboard has no text to paste.';
    if (text.length > maxBytes || new TextEncoder().encode(text).length > maxBytes) {
      throw `Clipboard text exceeds the ${maxBytes}-byte session limit.`;
    }
    return text;
  }, () => { throw 'Clipboard access was unavailable. Use your keyboard paste shortcut.'; });
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

// Canvas actions may run after browser user activation expires. Keep a real
// keyboard-accessible link available when opening a new tab is blocked.
export function openHelpLink(url, label, browser = globalThis.window, doc = globalThis.document) {
  let opened;
  try {
    opened = browser.open('about:blank', '_blank');
    if (opened) {
      opened.opener = null;
      opened.location.replace(url);
      return;
    }
  } catch { opened?.close(); }
  doc.getElementById('rmux-help-link')?.close();
  const origin = doc.activeElement;
  const dialog = doc.createElement('dialog');
  dialog.id = 'rmux-help-link';
  dialog.setAttribute('aria-labelledby', 'rmux-help-title');
  const title = doc.createElement('h2');
  title.id = 'rmux-help-title';
  title.textContent = label;
  const text = doc.createElement('p');
  text.textContent = 'Your browser did not open a new tab. Use this link to continue.';
  const link = doc.createElement('a');
  link.href = url;
  link.target = '_blank';
  link.rel = 'noopener noreferrer';
  link.textContent = label;
  const close = doc.createElement('button');
  close.textContent = 'Close';
  close.type = 'button';
  close.addEventListener('click', () => dialog.close());
  // Keep modal keystrokes out of the canvas/terminal handlers, including Tab
  // and Escape. The browser still handles native focus traversal and cancel.
  dialog.addEventListener('keydown', event => event.stopPropagation());
  dialog.addEventListener('keyup', event => event.stopPropagation());
  dialog.addEventListener('close', () => {
    dialog.remove();
    if (origin?.isConnected) origin.focus({preventScroll: true});
  }, {once: true});
  dialog.append(title, text, link, close);
  doc.body.append(dialog);
  dialog.showModal();
  link.focus();
}
