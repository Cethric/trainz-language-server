import * as vscode from 'vscode';
import { LanguageClient } from 'vscode-languageclient/node';

export function parseGsHoverResponse(response: any, client?: LanguageClient): vscode.Hover | null {
    if (!response) return null;
    if (client) {
        return client.protocol2CodeConverter.asHover(response) ?? null;
    }
    const contents = response.contents;
    if (contents && typeof contents === 'object' && 'value' in contents) {
        const markdownString = new vscode.MarkdownString(contents.value);
        markdownString.isTrusted = true;
        return new vscode.Hover(markdownString);
    }
    return new vscode.Hover(contents);
}

export function registerGsHoverProvider(
    _context: vscode.ExtensionContext,
    _clients: Map<string, LanguageClient>
) {
    // Hover is handled by the built-in vscode-languageclient HoverFeature.
    // Registering a separate provider here would cause duplicate hover results.
}
