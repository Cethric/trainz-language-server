import * as vscode from 'vscode';
import { LanguageClient } from 'vscode-languageclient/node';
import * as utils from '../../utils';

export function registerAcsDefinitionProvider(
    context: vscode.ExtensionContext,
    clients: Map<string, LanguageClient>
) {
    context.subscriptions.push(
        vscode.languages.registerDefinitionProvider(
            [{ scheme: 'file', language: 'acs' }],
            {
                provideDefinition: async (document, position, token) => {
                    const client = utils.getClientForDocument(document, clients);
                    if (!client) return null;
                    const response = await client.sendRequest<any>('textDocument/definition', {
                        textDocument: { uri: document.uri.toString() },
                        position: { line: position.line, character: position.character }
                    }, token);

                    if (!response) return null;
                    return client.protocol2CodeConverter.asDefinitionResult(response);
                }
            }
        )
    );
}
