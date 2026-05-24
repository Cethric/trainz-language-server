import * as vscode from 'vscode';
import { LanguageClient } from 'vscode-languageclient/node';
import * as utils from '../../utils';

export function parseGsHoverResponse(response: any): vscode.Hover | null {
    if (!response) return null;
    return new vscode.Hover(response.contents);
}

export function registerGsHoverProvider(
    context: vscode.ExtensionContext,
    clients: Map<string, LanguageClient>
) {
    context.subscriptions.push(
        vscode.languages.registerHoverProvider(
            [{ scheme: 'file', language: 'game-script' }],
            {
                provideHover: async (document, position, token) => {
                    const client = utils.getClientForDocument(document, clients);
                    if (!client) return null;
                    const response = await client.sendRequest<any>('textDocument/hover', {
                        textDocument: { uri: document.uri.toString() },
                        position: { line: position.line, character: position.character }
                    }, token);
                    
                    return parseGsHoverResponse(response);
                }
            }
        )
    );
}
