import * as vscode from 'vscode';
import { LanguageClient } from 'vscode-languageclient/node';
import * as utils from '../../utils';

function lspRangeToVscode(r: any): vscode.Range | undefined {
    if (!r || !r.start || !r.end) return undefined;
    return new vscode.Range(
        new vscode.Position(r.start.line, r.start.character),
        new vscode.Position(r.end.line, r.end.character)
    );
}

function convertSymbols(symbols: any[]): vscode.DocumentSymbol[] {
    if (!Array.isArray(symbols)) return [];
    const result: vscode.DocumentSymbol[] = [];
    for (const sym of symbols) {
        const range = lspRangeToVscode(sym.range);
        const selectionRange = lspRangeToVscode(sym.selectionRange) ?? range;
        if (!range || !selectionRange) continue;
        const ds = new vscode.DocumentSymbol(
            sym.name ?? '',
            sym.detail ?? '',
            sym.kind ?? vscode.SymbolKind.Variable,
            range,
            selectionRange
        );
        if (Array.isArray(sym.children) && sym.children.length > 0)
            ds.children = convertSymbols(sym.children);
        result.push(ds);
    }
    return result;
}

export function registerGsDocumentSymbols(
    context: vscode.ExtensionContext,
    clients: Map<string, LanguageClient>
) {
    context.subscriptions.push(
        vscode.languages.registerDocumentSymbolProvider(
            [{ scheme: 'file', language: 'game-script' }],
            {
                provideDocumentSymbols: async (document, token) => {
                    const client = utils.getClientForDocument(document, clients);
                    if (!client) return [];
                    const response = await client.sendRequest<any>('textDocument/documentSymbol', {
                        textDocument: { uri: document.uri.toString() }
                    }, token);
                    if (!response) return [];
                    return convertSymbols(response);
                }
            }
        )
    );
}
