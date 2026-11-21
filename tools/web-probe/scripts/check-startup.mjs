import test from 'node:test';
import assert from 'node:assert/strict';
import {startDemo} from '../boot.js';

function deferred() {
  let resolve, reject;
  const promise = new Promise((yes, no) => { resolve = yes; reject = no; });
  return {promise, resolve, reject};
}

test('loads concurrently but does not start the UI before both dependencies', async () => {
  const engine = deferred(), ui = deferred(), events = [];
  const module = {}, exports = {memory: {buffer: new ArrayBuffer(65536)}};
  const ready = startDemo({
    initializeEngine: () => { events.push('engine-load'); return engine.promise; },
    loadUi: () => { events.push('ui-load'); return ui.promise; },
    initializeUi: async options => { assert.equal(options.module_or_path, module); events.push('ui-start'); return exports; },
  });
  assert.deepEqual(events, ['engine-load', 'ui-load']);
  ui.resolve(module);
  await ui.promise;
  assert.equal(events.includes('ui-start'), false);
  engine.resolve();
  assert.equal(await ready, exports);
  assert.deepEqual(events, ['engine-load', 'ui-load', 'ui-start']);
});

for (const failure of ['engine', 'ui']) {
  test(`${failure} loading failure prevents UI startup`, async () => {
    let started = false;
    await assert.rejects(startDemo({
      initializeEngine: () => failure === 'engine' ? Promise.reject(new Error('engine failed')) : Promise.resolve(),
      loadUi: () => failure === 'ui' ? Promise.reject(new Error('ui failed')) : Promise.resolve({}),
      initializeUi: async () => { started = true; },
    }), /failed/);
    assert.equal(started, false);
  });
}
import {readTerminalClipboard} from '../engine.js';
test('terminal clipboard reads only bounded text during user activation', async () => {
  let called = false;
  const clipboard = {read() { throw new Error('must not request images'); }, readText() { called = true; return Promise.resolve('界🙂'); }};
  const read = readTerminalClipboard(7, clipboard);
  assert.equal(called, true);
  assert.equal(await read, '界🙂');
  await assert.rejects(readTerminalClipboard(6, clipboard), error => String(error).includes('session limit'));
  await assert.rejects(readTerminalClipboard(4, {readText: () => Promise.resolve('abcde')}));
  await assert.rejects(readTerminalClipboard(4096, {}));
  await assert.rejects(readTerminalClipboard(4096, {readText: () => Promise.reject(new Error('private browser detail'))}), error => !String(error).includes('private browser detail'));
});
