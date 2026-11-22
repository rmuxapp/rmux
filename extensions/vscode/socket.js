'use strict';
const net = require('node:net');
const path = require('node:path');
const os = require('node:os');

function socketPath(value) {
  if (typeof value !== 'string') throw new Error('Set rmux.socketPath to an absolute socket path.');
  const resolved = value.startsWith('~/') ? path.join(os.homedir(), value.slice(2)) : value;
  if (!path.isAbsolute(resolved) || resolved.includes('\0')) throw new Error('Set rmux.socketPath to an absolute socket path.');
  return resolved;
}

// One bounded local request per invocation. No polling, shell, or child process.
function openWorkspace(endpoint, folder, { timeout = 5000, signal } = {}) {
  return new Promise((resolve, reject) => {
    const socket = net.createConnection({ path: endpoint });
    let buffer = Buffer.alloc(0);
    let finished = false;
    const finish = (error, result) => {
      if (finished) return;
      finished = true;
      clearTimeout(timer);
      signal?.removeEventListener('abort', abort);
      socket.destroy();
      if (error) reject(error); else resolve(result);
    };
    const abort = () => finish(new Error('rmux request canceled.'));
    const timer = setTimeout(() => finish(new Error('rmux did not respond. Check that the app is running and try again.')), timeout);
    socket.on('error', error => finish(new Error(`Cannot reach rmux (${error.code || 'socket error'}). Start rmux and check rmux.socketPath and socket_directory.`)));
    socket.on('close', () => finish(new Error('rmux closed the connection before replying.')));
    socket.on('connect', () => socket.write(JSON.stringify({
      jsonrpc: '2.0', id: 1, method: 'workspace.open',
      params: { name: folder.name, cwd: folder.path, activate: true }
    }) + '\n'));
    socket.on('data', chunk => {
      if (buffer.length + chunk.length > 128 * 1024) return finish(new Error('rmux response exceeded the size limit.'));
      buffer = Buffer.concat([buffer, chunk]);
      const end = buffer.indexOf(10);
      if (end < 0) return;
      try {
        const response = JSON.parse(buffer.subarray(0, end).toString('utf8'));
        if (response.jsonrpc !== '2.0' || response.id !== 1) throw new Error('Unexpected rmux response.');
        if (response.error) throw new Error(String(response.error.message || 'rmux could not open this workspace.'));
        if (!response.result || typeof response.result.id !== 'string') throw new Error('Invalid rmux workspace response.');
        finish(null, response.result);
      } catch (error) { finish(error); }
    });
    signal?.addEventListener('abort', abort, { once: true });
    if (signal?.aborted) abort();
  });
}
module.exports = { openWorkspace, socketPath };
