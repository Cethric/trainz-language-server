import * as vscode from 'vscode';
import {LanguageClient} from 'vscode-languageclient/node';

export function registerGsDocumentSymbols(
    _context: vscode.ExtensionContext,
    _clients: Map<string, LanguageClient>
) {
    // Document symbols are handled by the built-in vscode-languageclient DocumentSymbolFeature.
    // Registering a separate provider here would cause duplicate results.
}
