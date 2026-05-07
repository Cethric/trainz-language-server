import * as vscode from 'vscode';
import {LanguageClient} from 'vscode-languageclient/node';
import {workspace} from 'vscode';
import * as utils from './utils';

export function registerGsFeatures(
    context: vscode.ExtensionContext,
    clients: Map<string, LanguageClient>,
    diagnosticCollection: vscode.DiagnosticCollection
) {
    const selector = [{scheme: 'file', language: 'game-script'}];

    // Register document symbol provider
    context.subscriptions.push(
        vscode.languages.registerDocumentSymbolProvider(
            selector,
            {
                provideDocumentSymbols: async (document, token) => {
                    const client = utils.getClientForDocument(document, clients);
                    if (!client || !client.isRunning()) {return [];}

                    try {
                        const response = await client.sendRequest<any>(
                            'textDocument/documentSymbol',
                            { textDocument: { uri: document.uri.toString() } },
                            token
                        );
                        return utils.convertDocumentSymbols(response, client, document);
                    } catch {
                        return [];
                    }
                }
            }
        )
    );

    // Register folding range provider
    context.subscriptions.push(
        vscode.languages.registerFoldingRangeProvider(
            selector,
            {
                provideFoldingRanges: async (document, context, token) => {
                    const client = utils.getClientForDocument(document, clients);
                    if (!client || !client.isRunning()) {return [];}

                    try {
                        const response = await client.sendRequest<any>(
                            'textDocument/foldingRange',
                            { textDocument: { uri: document.uri.toString() } },
                            token
                        );
                        if (!response) {return [];}

                        return response.map((range: any) => new vscode.FoldingRange(
                            range.startLine || 0,
                            range.endLine || 0,
                            range.kind as vscode.FoldingRangeKind | undefined
                        ));
                    } catch {
                        return [];
                    }
                }
            }
        )
    );

    // Register document semantic tokens provider
    context.subscriptions.push(
        vscode.languages.registerDocumentSemanticTokensProvider(
            selector,
            {
                provideDocumentSemanticTokens: async (document, token) => {
                    const client = utils.getClientForDocument(document, clients);
                    if (!client || !client.isRunning()) {return null;}

                    const folder = workspace.getWorkspaceFolder(document.uri);
                    if (!folder) {return null;}
                    const outerMost = utils.getOuterMostWorkspaceFolder(folder);
                    const mappings = utils.ensureSemanticTokenMappings(outerMost.uri.toString(), client);
                    if (!mappings) {return null;}

                    const { typeMapping, modifierMapping } = mappings;
                    try {
                        const response = await client.sendRequest<any>(
                            'textDocument/semanticTokens/full',
                            { textDocument: { uri: document.uri.toString() } },
                            token
                        );
                        if (!response || !response.data) {return null;}

                        const builder = new vscode.SemanticTokensBuilder(utils.semanticTokensLegend);
                        let i = 0;
                        while (i < response.data.length) {
                            const deltaLine = response.data[i++];
                            const deltaStart = response.data[i++];
                            const length = response.data[i++];
                            const tokenType = response.data[i++];
                            const tokenModifiers = response.data[i++];

                            const mappedType = typeMapping.get(tokenType) ?? 0;
                            let mappedMods = 0;
                            for (let j = 0; j < 32; j++) {
                                if (tokenModifiers & (1 << j)) {
                                    const mapped = modifierMapping.get(j);
                                    if (mapped !== undefined) {mappedMods |= (1 << mapped);}
                                }
                            }
                            builder.push(deltaLine, deltaStart, length, mappedType, mappedMods);
                        }
                        return builder.build();
                    } catch {
                        return null;
                    }
                }
            },
            utils.semanticTokensLegend
        )
    );
}
