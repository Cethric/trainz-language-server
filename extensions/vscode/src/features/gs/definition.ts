import * as vscode from 'vscode';
import { LanguageClient } from 'vscode-languageclient/node';
import * as utils from '../../utils';

export function parseGsDefinitionResponse(client: LanguageClient, response: any): any {
    if (!response) return null;
    return client.protocol2CodeConverter.asDefinitionResult(response);
}

export function registerGsDefinitionProvider(
    context: vscode.ExtensionContext,
    clients: Map<string, LanguageClient>
) {
    context.subscriptions.push(
        vscode.languages.registerDefinitionProvider(
            [{ scheme: 'file', language: 'game-script' }],
            {
                provideDefinition: async (document, position, token) => {
                    const client = utils.getClientForDocument(document, clients);
                    if (!client) return null;
                    const response = await client.sendRequest<any>('textDocument/definition', {
                        textDocument: { uri: document.uri.toString() },
                        position: { line: position.line, character: position.character }
                    }, token);

                    return parseGsDefinitionResponse(client, response);
                }
            }
        )
    );
}
