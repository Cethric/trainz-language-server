import * as vscode from 'vscode';
import {LanguageClient} from 'vscode-languageclient/node';
import {workspace} from 'vscode';
import * as utils from './utils';

export function registerAcsFeatures(
    context: vscode.ExtensionContext,
    clients: Map<string, LanguageClient>,
    diagnosticCollection: vscode.DiagnosticCollection
) {
    console.log(`[ACS EXTENSION] Registering ACS features`);
    const selector = [{scheme: 'file', language: 'acs'}];

    // Register document symbol provider
    context.subscriptions.push(
        vscode.languages.registerDocumentSymbolProvider(
            selector,
            {
                provideDocumentSymbols: async (document, token) => {
                    console.log(`[ACS EXTENSION] Document symbols requested for: ${document.fileName}`);
                    const client = utils.getClientForDocument(document, clients);
                    if (!client || !client.isRunning()) {
                        console.log(`[ACS EXTENSION] Client not found or not running for: ${document.fileName}`);
                        return [];
                    }

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
                    console.log(`[ACS EXTENSION] Folding ranges requested for: ${document.fileName}`);
                    const client = utils.getClientForDocument(document, clients);
                    if (!client || !client.isRunning()) {
                        console.log(`[ACS EXTENSION] Client not found or not running for: ${document.fileName}`);
                        return [];
                    }

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
                    console.log(`[ACS EXTENSION] Semantic tokens requested for: ${document.fileName}`);
                    const client = utils.getClientForDocument(document, clients);
                    if (!client || !client.isRunning()) {
                        console.log(`[ACS EXTENSION] Client not found or not running for: ${document.fileName}`);
                        return null;
                    }
                    console.log(`[ACS EXTENSION] Client found for: ${document.fileName}. Proceeding to fetch semantic tokens.`);

                    const folder = workspace.getWorkspaceFolder(document.uri);
                    if (!folder) {
                        return null;
                    }
                    const outerMost = utils.getOuterMostWorkspaceFolder(folder);
                    const mappings = utils.ensureSemanticTokenMappings(outerMost.uri.toString(), client);
                    if (!mappings) {
                        return null;
                    }

                    const { typeMapping, modifierMapping } = mappings;
                    try {
                        console.log(`[ACS EXTENSION] Sending semantic tokens request for: ${document.fileName}`);
                        const response = await client.sendRequest<any>(
                            'textDocument/semanticTokens/full',
                            { textDocument: { uri: document.uri.toString() } },
                            token
                        );
                        console.log(`[ACS EXTENSION] Received semantic tokens response for: ${document.fileName}. Response exists: ${!!response}, Data length: ${response?.data?.length}`);
                        console.log(`[ACS EXTENSION] Response data: ${JSON.stringify(response?.data)}`);
                        if (!response || !response.data) {
                            return null;
                        }

                        const builder = new vscode.SemanticTokensBuilder(utils.semanticTokensLegend);
                        if (response.data.length > 0 && typeof response.data[0] === 'object') {
                            console.log(`[ACS EXTENSION] Handling response as array of objects`);
                            for (const token of response.data) {
                                const deltaLine = token.deltaLine ?? token.delta_line ?? 0;
                                const deltaStart = token.deltaStart ?? token.delta_start ?? 0;
                                const length = token.length ?? 0;
                                const tokenType = token.tokenType ?? token.token_type ?? 0;
                                const tokenModifiers = token.tokenModifiers ?? token.token_modifiers_bitset ?? 0;

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
                        } else {
                            console.log(`[ACS EXTENSION] Handling response as flat array of numbers`);
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
                        }
                        return builder.build();
                    } catch (e) {
                        console.error(`Error in semantic tokens provider: ${e}`);
                        return null;
                    }
                }
            },
            utils.semanticTokensLegend
        )
    );
}
