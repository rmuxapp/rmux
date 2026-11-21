// Load independent WASM modules concurrently; start the UI only after the VT
// host exists. Timing measures initialization, not first paint or input latency.
export async function loadUiModule(url) {
  const response = await fetch(url);
  if (!response.ok) throw new Error(`Loading rmux failed: ${response.status}`);
  if (typeof WebAssembly.compileStreaming === 'function' &&
      response.headers.get('Content-Type')?.split(';')[0].trim() === 'application/wasm') {
    return WebAssembly.compileStreaming(response);
  }
  return WebAssembly.compile(await response.arrayBuffer());
}

export async function startDemo({initializeEngine, initializeUi, loadUi = () =>
    loadUiModule(new URL('./rmux_web_probe_bg.wasm', import.meta.url))}) {
  performance.mark('rmux-init-start');
  const [, module] = await Promise.all([
    initializeEngine().then(() => performance.mark('rmux-engine-ready')),
    loadUi().then(module => { performance.mark('rmux-ui-compiled'); return module; }),
  ]);
  await initializeUi({module_or_path: module});
  performance.mark('rmux-init-end');
  performance.measure('rmux-initialization', 'rmux-init-start', 'rmux-init-end');
}
