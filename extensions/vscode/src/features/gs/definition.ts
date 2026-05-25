import * as vscode from 'vscode';
import {LanguageClient} from 'vscode-languageclient/node';

export function parseGsDefinitionResponse(client: LanguageClient, response: any): any {
    if (!response) return null;
    return client.protocol2CodeConverter.asDefinitionResult(response);
}

export function registerGsDefinitionProvider(
    _context: vscode.ExtensionContext,
    _clients: Map<string, LanguageClient>
) {
    // Definition is handled by the built-in vscode-languageclient DefinitionFeature.
    // Registering a separate provider here would cause duplicate results.
}
