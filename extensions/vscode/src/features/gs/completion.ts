import * as vscode from 'vscode';
import { LanguageClient } from 'vscode-languageclient/node';
import * as utils from '../../utils';

export function parseGsCompletionResponse(response: any): vscode.CompletionItem[] {
    if (!response) return [];
    const items = response.items || response;
    return items.map((item: any) => new vscode.CompletionItem(item.label));
}

export function registerGsCompletionProvider(
    context: vscode.ExtensionContext,
    clients: Map<string, LanguageClient>
) {
    context.subscriptions.push(
        vscode.languages.registerCompletionItemProvider(
            [{ scheme: 'file', language: 'game-script' }],
            {
                provideCompletionItems: async (document, position, token) => {
                    const client = utils.getClientForDocument(document, clients);
                    if (!client) return [];
                    const response = await client.sendRequest<any>('textDocument/completion', {
                        textDocument: { uri: document.uri.toString() },
                        position: { line: position.line, character: position.character }
                    }, token);

                    return parseGsCompletionResponse(response);
                }
            },
            '.'
        )
    );
}
