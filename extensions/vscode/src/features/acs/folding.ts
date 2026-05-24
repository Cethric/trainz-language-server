import * as vscode from 'vscode';
import { LanguageClient } from 'vscode-languageclient/node';
import * as utils from '../../utils';

export function registerAcsFoldingRanges(
    context: vscode.ExtensionContext,
    clients: Map<string, LanguageClient>
) {
    context.subscriptions.push(
        vscode.languages.registerFoldingRangeProvider(
            [{ scheme: 'file', language: 'acs' }],
            {
                provideFoldingRanges: async (document, _context, token) => {
                    const client = utils.getClientForDocument(document, clients);
                    if (!client) return [];
                    const response = await client.sendRequest<any>('textDocument/foldingRange', {
                        textDocument: { uri: document.uri.toString() }
                    }, token);
                    return response;
                }
            }
        )
    );
}
