import * as path from 'path';
import { workspace, ExtensionContext } from 'vscode';
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  Executable
} from 'vscode-languageclient/node';

let client: LanguageClient;

export function activate(context: ExtensionContext) {
  const config = workspace.getConfiguration('gs-lsp');
  let command = config.get<string>('serverPath') || 'gs-lsp';

  const run: Executable = {
    command,
    options: {
      env: { ...process.env, RUST_LOG: 'info' }
    }
  };

  const serverOptions: ServerOptions = {
    run,
    debug: run
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [
      { scheme: 'file', language: 'game-script' }, 
      { scheme: 'file', pattern: '**/*.gs', language: 'game-script' },
      { scheme: 'file', language: 'soup' },
      { scheme: 'file', pattern: '**/*.txt', language: 'soup' }
    ],

    synchronize: {
      fileEvents: workspace.createFileSystemWatcher('**/.clientrc')
    }
  };

  client = new LanguageClient(
    'gsLanguageServer',
    'GameScript Language Server',
    serverOptions,
    clientOptions
  );

  client.start();
}

export function deactivate(): Thenable<void> | undefined {
  if (!client) {
    return undefined;
  }
  return client.stop();
}
