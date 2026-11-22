'use strict';
const { openWorkspace, socketPath } = require('./socket');

function register(vscode, context) {
  let pending;
  const controller = new AbortController();
  context.subscriptions.push({ dispose: () => controller.abort() });
  context.subscriptions.push(vscode.commands.registerCommand('rmux.openProject', () => {
    // Key repeat must not spawn duplicate requests or folder pickers.
    if (pending) return pending;
    pending = (async () => {
      if (!vscode.workspace.isTrusted) throw new Error('Trust this workspace before opening its terminal.');
      if (process.platform === 'win32' || vscode.env.remoteName) throw new Error('This integration currently supports local macOS and Linux projects.');
      const folders = vscode.workspace.workspaceFolders || [];
      if (!folders.length) throw new Error('Open a project folder first.');
      const active = vscode.window.activeTextEditor?.document.uri;
      let folder = active && vscode.workspace.getWorkspaceFolder(active);
      if (!folder && folders.length === 1) folder = folders[0];
      if (!folder) folder = await vscode.window.showWorkspaceFolderPick({ placeHolder: 'Open which project in rmux?' });
      if (!folder || controller.signal.aborted) return;
      if (folder.uri.scheme !== 'file') throw new Error('rmux needs a local project folder.');
      const endpoint = socketPath(vscode.workspace.getConfiguration('rmux').get('socketPath'));
      await openWorkspace(endpoint, { name: folder.name, path: folder.uri.fsPath }, { signal: controller.signal });
    })().catch(error => {
      if (!controller.signal.aborted) return vscode.window.showErrorMessage(error.message);
    }).finally(() => { pending = undefined; });
    return pending;
  }));
}
function activate(context) { register(require('vscode'), context); }
module.exports = { activate, register };
