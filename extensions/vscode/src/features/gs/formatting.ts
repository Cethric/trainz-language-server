import * as vscode from 'vscode';
import { LanguageClient } from 'vscode-languageclient/node';
import * as utils from '../../utils';

export function registerGsFormattingProvider(
    context: vscode.ExtensionContext,
    clients: Map<string, LanguageClient>
) {
    context.subscriptions.push(
        vscode.languages.registerDocumentFormattingEditProvider(
            [{ scheme: 'file', language: 'game-script' }],
            {
                provideDocumentFormattingEdits: async (document, options, token) => {
                    const client = utils.getClientForDocument(document, clients);
                    if (!client) return null;
                    const response = await client.sendRequest<any>('textDocument/formatting', {
                        textDocument: { uri: document.uri.toString() },
                        options: options
                    }, token);

                    if (!response) return null;
                    return client.protocol2CodeConverter.asTextEdits(response);
                }
            }
        )
    );
}
