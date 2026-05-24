import * as vscode from 'vscode';
import { LanguageClient, LanguageClientOptions, ServerOptions } from 'vscode-languageclient/node';
import { registerGsDocumentSymbols } from './features/gs/symbols';
import { registerAcsDocumentSymbols } from './features/acs/symbols';
import { registerGsFoldingRanges } from './features/gs/folding';
import { registerAcsFoldingRanges } from './features/acs/folding';
import { registerGsCompletionProvider } from './features/gs/completion';
import { registerAcsCompletionProvider } from './features/acs/completion';
import { registerGsHoverProvider } from './features/gs/hover';
import { registerAcsHoverProvider } from './features/acs/hover';
import { registerGsSemanticTokens } from './features/gs/semanticTokens';
import { registerAcsSemanticTokens } from './features/acs/semanticTokens';
import { registerGsDefinitionProvider } from './features/gs/definition';
import { registerAcsDefinitionProvider } from './features/acs/definition';
import { registerGsFormattingProvider } from './features/gs/formatting';
import { registerAcsFormattingProvider } from './features/acs/formatting';

const clients: Map<string, LanguageClient> = new Map();

export function getClients(): Map<string, LanguageClient> {
    return clients;
}

export async function activate(context: vscode.ExtensionContext) {
    console.log('[Trainz LSP] Activating extension...');

    const traceChannel = vscode.window.createOutputChannel('Trainz LSP Trace');
    const outputChannel = vscode.window.createOutputChannel('Trainz LSP Output');

    function didOpenTextDocument(document: vscode.TextDocument): void {
        // We are only interested in language mode text
        console.log('[Trainz LSP] Document opened:', document.uri.toString(), 'Language ID:', document.languageId);
        if ((document.languageId !== 'game-script' && document.languageId !== 'acs') ||
            (document.uri.scheme !== 'file' && document.uri.scheme !== 'untitled')) {
            return;
        }

        const folder = vscode.workspace.getWorkspaceFolder(document.uri);
        const folderKey = folder ? folder.uri.toString() : 'default';

        if (!clients.has(folderKey)) {
            console.log('[Trainz LSP] Starting client for folder', folderKey);
            const config = vscode.workspace.getConfiguration('trainz-language-server', folder ? folder.uri : null);
            const serverBin = config.get<string>('server-bin') || 'trainz-language-server';

            const serverOptions: ServerOptions = {
                command: serverBin,
                args: [],
            };

            const clientOptions: LanguageClientOptions = {
                documentSelector: [
                    { scheme: 'file', language: 'game-script', pattern: '**/*.gs' },
                    { scheme: 'file', language: 'acs' }
                ],
                workspaceFolder: folder,
                // The built-in LSP client handles textDocument/didOpen, didChange, didClose
                // synchronization automatically. File watcher events are sent separately
                // as workspace/didChangeWatchedFiles notifications for background indexing.
                synchronize: {
                    fileEvents: vscode.workspace.createFileSystemWatcher('**/*')
                },
                progressOnInitialization: true,
                traceOutputChannel: traceChannel,
                outputChannel: outputChannel,
            };

            const client = new LanguageClient(
                'trainz-language-server',
                'Trainz Language Server',
                serverOptions,
                clientOptions
            );

            client.start();
            clients.set(folderKey, client);
        }
    }

    // Register all providers with the clients map
    registerGsDocumentSymbols(context, clients);
    registerAcsDocumentSymbols(context, clients);
    registerGsFoldingRanges(context, clients);
    registerAcsFoldingRanges(context, clients);
    registerGsCompletionProvider(context, clients);
    registerAcsCompletionProvider(context, clients);
    registerGsHoverProvider(context, clients);
    registerAcsHoverProvider(context, clients);
    registerGsSemanticTokens(context, clients);
    registerAcsSemanticTokens(context, clients);
    registerGsDefinitionProvider(context, clients);
    registerAcsDefinitionProvider(context, clients);
    registerGsFormattingProvider(context, clients);
    registerAcsFormattingProvider(context, clients);

    vscode.workspace.onDidOpenTextDocument(didOpenTextDocument);
    vscode.workspace.textDocuments.forEach(didOpenTextDocument);
    
    console.log('[Trainz LSP] Extension activated.');
}

export function deactivate(): Thenable<void> | undefined {
    const promises: Thenable<void>[] = [];
    for (const client of clients.values()) {
        promises.push(client.stop());
    }
    return Promise.all(promises).then(() => undefined);
}
