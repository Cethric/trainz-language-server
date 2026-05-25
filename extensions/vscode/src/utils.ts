import * as vscode from 'vscode';
import {LanguageClient} from 'vscode-languageclient/node';

export function getClientForDocument(
    document: vscode.TextDocument,
    clients: Map<string, LanguageClient>
): LanguageClient | undefined {
    const workspaceFolder = vscode.workspace.getWorkspaceFolder(document.uri);
    if (workspaceFolder) {
        const client = clients.get(workspaceFolder.uri.toString());
        if (client) return client;
    }
    // Fallback: check default key (used when no workspace folder is open)
    const defaultClient = clients.get('default');
    if (defaultClient) return defaultClient;
    // Last resort: return the only client if there's just one
    if (clients.size === 1) {
        return clients.values().next().value;
    }
    return undefined;
}
