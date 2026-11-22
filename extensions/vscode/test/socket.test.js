'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const net = require('node:net');
const fs = require('node:fs/promises');
const os = require('node:os');
const path = require('node:path');
const { once } = require('node:events');
const { openWorkspace, socketPath } = require('../socket');
const { register } = require('../extension');

async function server(t, handler) {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), 'rmx-'));
  const endpoint = path.join(root, 'rpc.sock');
  const sockets = new Set();
  const listener = net.createServer(socket => {
    sockets.add(socket);
    socket.on('error', () => {});
    socket.on('close', () => sockets.delete(socket));
    handler(socket);
  });
  listener.listen(endpoint);
  await once(listener, 'listening');
  t.after(async () => {
    for (const socket of sockets) socket.destroy();
    await new Promise(resolve => listener.close(resolve));
    await fs.rm(root, { recursive: true, force: true });
  });
  return endpoint;
}

test('workspace request preserves project data and handles a split UTF-8 reply', async t => {
  const endpoint = await server(t, socket => socket.once('data', bytes => {
    assert.deepEqual(JSON.parse(bytes), { jsonrpc: '2.0', id: 1, method: 'workspace.open',
      params: { name: '世界 project', cwd: '/tmp/space $(not-a-shell)', activate: true } });
    const reply = Buffer.from(JSON.stringify({ jsonrpc: '2.0', id: 1, result: { id: '42', name: '世界' } }) + '\n');
    const split = reply.indexOf(Buffer.from('世界')) + 1;
    socket.write(reply.subarray(0, split));
    setImmediate(() => socket.end(reply.subarray(split)));
  }));
  assert.equal((await openWorkspace(endpoint, { name: '世界 project', path: '/tmp/space $(not-a-shell)' })).name, '世界');
});

test('errors, premature disconnects, malformed replies, and oversized replies reject', async t => {
  for (const [reply, message] of [
    [JSON.stringify({ jsonrpc: '2.0', id: 1, error: { message: 'Workspace name is ambiguous' } }) + '\n', /ambiguous/],
    ['not json\n', /JSON|Unexpected/],
    [JSON.stringify({ jsonrpc: '2.0', id: 2, result: {} }) + '\n', /Unexpected/],
    ['', /closed/], ['x'.repeat(128 * 1024 + 1), /size limit/]
  ]) {
    const endpoint = await server(t, socket => socket.once('data', () => socket.end(reply)));
    await assert.rejects(openWorkspace(endpoint, { name: 'project', path: '/tmp' }), message);
  }
});

test('silent server has an absolute deadline and cancellation closes the request', async t => {
  const endpoint = await server(t, () => {});
  await assert.rejects(openWorkspace(endpoint, { name: 'project', path: '/tmp' }, { timeout: 30 }), /did not respond/);
  const controller = new AbortController();
  const pending = openWorkspace(endpoint, { name: 'project', path: '/tmp' }, { signal: controller.signal });
  controller.abort();
  await assert.rejects(pending, /canceled/);
  assert.equal(socketPath('~/rpc.sock'), path.join(os.homedir(), 'rpc.sock'));
  assert.throws(() => socketPath('relative.sock'), /absolute/);
});

test('direct command chooses the active project and coalesces key repeat', async t => {
  let requests = 0;
  const endpoint = await server(t, socket => socket.once('data', data => {
    requests++;
    assert.equal(JSON.parse(data).params.name, 'second');
    socket.end('{"jsonrpc":"2.0","id":1,"result":{"id":"42"}}\n');
  }));
  const folder = { name: 'second', uri: { scheme: 'file', fsPath: '/tmp/second' } };
  let command;
  const context = { subscriptions: [] };
  const vscode = {
    env: {},
    commands: { registerCommand: (_, handler) => { command = handler; return { dispose() {} }; } },
    workspace: { isTrusted: true, workspaceFolders: [{ name: 'first' }, folder],
      getWorkspaceFolder: () => folder, getConfiguration: () => ({ get: () => endpoint }) },
    window: { activeTextEditor: { document: { uri: {} } },
      showWorkspaceFolderPick: () => assert.fail('Active project should not need a picker'),
      showErrorMessage: message => assert.fail(message) }
  };
  register(vscode, context);
  const first = command();
  assert.equal(command(), first);
  await first;
  assert.equal(requests, 1);
  vscode.env.remoteName = 'ssh-remote';
  let error;
  vscode.window.showErrorMessage = message => { error = message; };
  await command();
  assert.match(error, /local macOS and Linux/);
  assert.equal(requests, 1);
  for (const subscription of context.subscriptions) subscription.dispose();
});
