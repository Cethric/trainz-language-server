import * as vscode from 'vscode';
import { LanguageClient } from 'vscode-languageclient/node';

export function registerAcsHoverProvider(
    _context: vscode.ExtensionContext,
    _clients: Map<string, LanguageClient>
) {
    // Hover is handled by the built-in vscode-languageclient HoverFeature.
    // Registering a separate provider here would cause duplicate hover results.
}
