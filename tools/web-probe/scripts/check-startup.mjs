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
  const module = {};
  const ready = startDemo({
    initializeEngine: () => { events.push('engine-load'); return engine.promise; },
    loadUi: () => { events.push('ui-load'); return ui.promise; },
    initializeUi: async options => { assert.equal(options.module_or_path, module); events.push('ui-start'); },
  });
  assert.deepEqual(events, ['engine-load', 'ui-load']);
  ui.resolve(module);
  await ui.promise;
  assert.equal(events.includes('ui-start'), false);
  engine.resolve();
  await ready;
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
