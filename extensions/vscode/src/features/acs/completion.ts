import * as vscode from 'vscode';
import { LanguageClient } from 'vscode-languageclient/node';

export function parseAcsCompletionResponse(response: any): vscode.CompletionItem[] {
    if (!response) return [];
    const items = response.items || response;
    return items.map((item: any) => new vscode.CompletionItem(item.label));
}

export function registerAcsCompletionProvider(
    _context: vscode.ExtensionContext,
    _clients: Map<string, LanguageClient>
) {
    // Completions are handled by the built-in vscode-languageclient CompletionFeature.
    // Registering a separate provider here would cause duplicate results.
}
