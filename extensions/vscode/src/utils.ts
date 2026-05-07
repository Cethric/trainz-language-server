import * as vscode from 'vscode';
import {LanguageClient} from 'vscode-languageclient/node';
import {WorkspaceFolder, workspace} from 'vscode';

export const symbolKindMappings = new Map<string, Map<number, vscode.SymbolKind>>();
export const tokenTypeMappings = new Map<string, Map<number, number>>();
export const tokenModifierMappings = new Map<string, Map<number, number>>();

export function ensureSymbolKindMapping(folderKey: string, client: LanguageClient): Map<number, vscode.SymbolKind> | undefined {
    if (symbolKindMappings.has(folderKey)) {
        return symbolKindMappings.get(folderKey);
    }

    const serverCapabilities = client.initializeResult?.capabilities;
    if (serverCapabilities?.documentSymbolProvider && (serverCapabilities.documentSymbolProvider as any).legend && (serverCapabilities.documentSymbolProvider as any).legend.symbolKinds) {
        const legend = (serverCapabilities.documentSymbolProvider as any).legend;
        const mapping = new Map<number, vscode.SymbolKind>();
        legend.symbolKinds.forEach((kind: string, index: number) => {
            const vscKind = (vscode.SymbolKind as any)[kind.charAt(0).toUpperCase() + kind.slice(1)];
            if (vscKind !== undefined) {
                mapping.set(index + 1, vscKind);
            }
        });
        symbolKindMappings.set(folderKey, mapping);
        return mapping;
    }
    return undefined;
}

export function ensureSemanticTokenMappings(folderKey: string, client: LanguageClient): { typeMapping: Map<number, number>; modifierMapping: Map<number, number> } | undefined {
    if (tokenTypeMappings.has(folderKey) && tokenModifierMappings.has(folderKey)) {
        return {
            typeMapping: tokenTypeMappings.get(folderKey)!,
            modifierMapping: tokenModifierMappings.get(folderKey)!
        };
    }

    const serverCapabilities = client.initializeResult?.capabilities;
    if (serverCapabilities?.semanticTokensProvider && serverCapabilities.semanticTokensProvider.legend) {
        const serverLegend = serverCapabilities.semanticTokensProvider.legend;
        const clientTokenTypes = [
            'namespace', 'type', 'class', 'enum', 'interface', 'struct', 'typeParameter',
            'parameter', 'variable', 'property', 'enumMember', 'event', 'function',
            'method', 'macro', 'keyword', 'modifier', 'comment', 'string', 'number',
            'regexp', 'operator'
        ];
        const clientTokenModifiers = [
            'declaration', 'definition', 'readonly', 'static', 'deprecated', 'abstract',
            'async', 'modification', 'documentation', 'defaultLibrary'
        ];

        const typeMapping = new Map<number, number>();
        serverLegend.tokenTypes.forEach((type, index) => {
            const clientIndex = clientTokenTypes.indexOf(type);
            if (clientIndex !== -1) {
                typeMapping.set(index, clientIndex);
            }
        });

        const modifierMapping = new Map<number, number>();
        serverLegend.tokenModifiers.forEach((mod, index) => {
            const clientIndex = clientTokenModifiers.indexOf(mod);
            if (clientIndex !== -1) {
                modifierMapping.set(index, clientIndex);
            }
        });

        tokenTypeMappings.set(folderKey, typeMapping);
        tokenModifierMappings.set(folderKey, modifierMapping);

        return { typeMapping, modifierMapping };
    }
    return undefined;
}

export const semanticTokensLegend = {
  tokenTypes: [
    'namespace', 'type', 'class', 'enum', 'interface', 'struct', 'typeParameter',
    'parameter', 'variable', 'property', 'enumMember', 'event', 'function',
    'method', 'macro', 'keyword', 'modifier', 'comment', 'string', 'number',
    'regexp', 'operator'
  ],
  tokenModifiers: [
    'declaration', 'definition', 'readonly', 'static', 'deprecated', 'abstract',
    'async', 'modification', 'documentation', 'defaultLibrary'
  ]
};

let _sortedWorkspaceFolders: string[] | undefined;

export function sortedWorkspaceFolders(): string[] {
    if (_sortedWorkspaceFolders === void 0) {
        _sortedWorkspaceFolders = workspace.workspaceFolders ? workspace.workspaceFolders.map(folder => {
            let result = folder.uri.toString();
            if (result.charAt(result.length - 1) !== '/') {
                result = result + '/';
            }
            return result;
        }).sort(
            (a, b) => {
                return a.length - b.length;
            }
        ) : [];
    }
    return _sortedWorkspaceFolders;
}

workspace.onDidChangeWorkspaceFolders(() => _sortedWorkspaceFolders = undefined);

export function getOuterMostWorkspaceFolder(folder: WorkspaceFolder): WorkspaceFolder {
    const sorted = sortedWorkspaceFolders();
    for (const element of sorted) {
        let uri = folder.uri.toString();
        if (uri.charAt(uri.length - 1) !== '/') {
            uri = uri + '/';
        }
        if (uri.startsWith(element)) {
            return workspace.getWorkspaceFolder(vscode.Uri.parse(element))!;
        }
    }
    return folder;
}

export function getClientForDocument(document: vscode.TextDocument, clients: Map<string, LanguageClient>): LanguageClient | undefined {
    const uri = document.uri;

    // Untitled files go to a default client (if we had one)
    if (uri.scheme === 'untitled') {
        return undefined; // This LSP doesn't support untitled files
    }

    let folder = workspace.getWorkspaceFolder(uri);
    if (!folder) {
        return undefined;
    }

    // If we have nested workspace folders, we only start a server on the outer most workspace folder
    folder = getOuterMostWorkspaceFolder(folder);
    return clients.get(folder.uri.toString());
}

/**
 * Convert LSP document symbols to VS Code format
 */
export function convertDocumentSymbols(
    symbols: any[],
    client: LanguageClient,
    document: vscode.TextDocument
): vscode.DocumentSymbol[] {
    const folder = workspace.getWorkspaceFolder(document.uri);
    let mapping: Map<number, vscode.SymbolKind> | undefined;
    if (folder) {
        const outerMost = getOuterMostWorkspaceFolder(folder);
        const folderKey = outerMost.uri.toString();
        mapping = ensureSymbolKindMapping(folderKey, client);
    }

    return symbols.map((symbol) => {
        const convertedSymbol = new vscode.DocumentSymbol(
            symbol.name,
            symbol.detail || '',
            convertSymbolKind(symbol.kind, mapping),
            client.protocol2CodeConverter.asRange(symbol.range) ||
            new vscode.Range(0, 0, 0, 0),
            client.protocol2CodeConverter.asRange(symbol.selectionRange) ||
            new vscode.Range(0, 0, 0, 0)
        );

        // Recursively convert children
        if (symbol.children && Array.isArray(symbol.children)) {
            convertedSymbol.children = convertDocumentSymbols(symbol.children, client, document);
        }

        return convertedSymbol;
    });
}

/**
 * Convert LSP symbol kind to VS Code symbol kind
 */
export function convertSymbolKind(lspKind: number, mapping?: Map<number, vscode.SymbolKind>): vscode.SymbolKind {
    if (mapping) {
        return mapping.get(lspKind) || vscode.SymbolKind.Variable;
    }
    const kindMap: { [key: number]: vscode.SymbolKind } = {
        1: vscode.SymbolKind.Class,
        2: vscode.SymbolKind.Method,
        3: vscode.SymbolKind.Property,
        4: vscode.SymbolKind.Field,
        5: vscode.SymbolKind.Variable,
        6: vscode.SymbolKind.String,
        7: vscode.SymbolKind.Number,
        8: vscode.SymbolKind.Key,
        9: vscode.SymbolKind.Operator,
        10: vscode.SymbolKind.TypeParameter,
    };

    return kindMap[lspKind] || vscode.SymbolKind.Variable;
}
