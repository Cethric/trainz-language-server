import * as vscode from 'vscode';
import {ExtensionContext, languages, LogOutputChannel, Uri, window, workspace, WorkspaceFolder, SemanticTokens, SemanticTokensBuilder, SemanticTokensLegend} from 'vscode';
import {Executable, LanguageClient, LanguageClientOptions, ServerOptions} from 'vscode-languageclient/node';

// let defaultClient: LanguageClient;
const clients = new Map<string, LanguageClient>();

const symbolKindMappings = new Map<string, Map<number, vscode.SymbolKind>>();
const tokenTypeMappings = new Map<string, Map<number, number>>();
const tokenModifierMappings = new Map<string, Map<number, number>>();

const semanticTokensLegend: SemanticTokensLegend = {
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

function sortedWorkspaceFolders(): string[] {
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

function getWorkspaceFolderKey(folder: WorkspaceFolder): string {
    let key = folder.uri.toString();
    if (!key.endsWith('/')) {
        key = `${key}/`;
    }
    return key;
}

function getOuterMostWorkspaceFolder(folder: WorkspaceFolder): WorkspaceFolder {
    const sorted = sortedWorkspaceFolders();
    for (const element of sorted) {
        let uri = folder.uri.toString();
        if (uri.charAt(uri.length - 1) !== '/') {
            uri = uri + '/';
        }
        if (uri.startsWith(element)) {
            return workspace.getWorkspaceFolder(Uri.parse(element))!;
        }
    }
    return folder;
}

function getClientForDocument(document: vscode.TextDocument): LanguageClient | undefined {
    const folder = workspace.getWorkspaceFolder(document.uri);
    if (!folder) {
        return undefined;
    }

    const outerMostFolder = getOuterMostWorkspaceFolder(folder);
    return clients.get(getWorkspaceFolderKey(outerMostFolder));
}

export function activate(context: ExtensionContext) {
    const config = workspace.getConfiguration('trainz-language-server');
    let command = config.get<string>('server-bin') || 'lsp-bin';
    let validation = config.get<string>('validation-path') || undefined;
    let search = config.get<string[]>('search-paths') || undefined;


    const outputChannel: LogOutputChannel = window.createOutputChannel('trainz-language-server', {log: true});
    const traceOutputChannel = window.createOutputChannel('trainz-language-server trace', {log: true});
    context.subscriptions.push(traceOutputChannel);

    const run: Executable = {
        command,
        // transport: TransportKind.stdio,
        options: {
            env: {
                ...process.env,
                TRAINZ_LANGUAGE_SERVER_SCRIPT_SEARCH_PATHS: search?.join(";"),
                TRAINZ_LANGUAGE_SERVER_SOUP_VALIDATION_PATH: validation,
                RUST_LOG: "debug"
            }
        }
    };

    const serverOptions: ServerOptions = {
        run,
        debug: run
    };

    const clientOptions: LanguageClientOptions = {
        documentSelector: [
            {scheme: 'file', pattern: '**/*.gs', language: 'game-script'},
            {scheme: 'file', pattern: '**/*.txt', language: 'soup'}
        ],

        synchronize: {
            fileEvents: workspace.createFileSystemWatcher('**/.clientrc')
        },

        outputChannel,
        traceOutputChannel,
        stdioEncoding: 'utf8',
        progressOnInitialization: true,
        initializationOptions: {
            capabilities: {
                textDocument: {
                    semanticTokens: {
                        dynamicRegistration: false,
                        legend: semanticTokensLegend
                    }
                }
            }
        }
    };

    const diagnosticCollection = languages.createDiagnosticCollection('trainz-language-server');
    context.subscriptions.push(diagnosticCollection);

    function didOpenTextDocument(document: vscode.TextDocument) {
        let folder = workspace.getWorkspaceFolder(document.uri);
        if (!folder) {
            return;
        }

        folder = getOuterMostWorkspaceFolder(folder);
        const folderKey = getWorkspaceFolderKey(folder);

        if (!clients.has(folderKey)) {
            const folderClientOptions: LanguageClientOptions = {
                ...clientOptions,
                workspaceFolder: folder
            };

            const client = new LanguageClient(
                'trainz-language-server',
                `Trainz Language Server (${folder.name})`,
                serverOptions,
                folderClientOptions
            );

            client.start().then(() => {
                client.onNotification(
                    'textDocument/publishDiagnostics',
                    (params: any) => {
                        handleDiagnostics(client, diagnosticCollection, params);
                    }
                );

                // Get server capabilities and create mappings
                const serverCapabilities = client.initializeResult!.capabilities;

                // For symbol kinds
                if (serverCapabilities.documentSymbolProvider && (serverCapabilities.documentSymbolProvider as any).legend && (serverCapabilities.documentSymbolProvider as any).legend.symbolKinds) {
                    const legend = (serverCapabilities.documentSymbolProvider as any).legend;
                    const mapping = new Map<number, vscode.SymbolKind>();
                    legend.symbolKinds.forEach((kind: string, index: number) => {
                        const vscKind = (vscode.SymbolKind as any)[kind.charAt(0).toUpperCase() + kind.slice(1)];
                        if (vscKind !== undefined) {
                            mapping.set(index + 1, vscKind);
                        }
                    });
                    symbolKindMappings.set(folderKey, mapping);
                }

                // For semantic tokens
                if (serverCapabilities.semanticTokensProvider && serverCapabilities.semanticTokensProvider.legend) {
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
                }
            }).catch((error) => {
                console.error('Failed to start language client for', folder.uri.toString(), error);
            });

            clients.set(folderKey, client);
            context.subscriptions.push(client);
        }
    }

    function handleDocumentClose(document: vscode.TextDocument) {
        diagnosticCollection.delete(document.uri);
    }

    registerLanguageFeatures(context);

    workspace.onDidOpenTextDocument(didOpenTextDocument);
    workspace.onDidCloseTextDocument(handleDocumentClose);
    workspace.textDocuments.forEach(didOpenTextDocument);

    workspace.onDidChangeWorkspaceFolders((event) => {
        for (const folder of event.removed) {
            const folderKey = getWorkspaceFolderKey(folder);
            const client = clients.get(folderKey);
            if (client) {
                clients.delete(folderKey);
                client.stop().catch(console.error);
            }

            symbolKindMappings.delete(folderKey);
            tokenTypeMappings.delete(folderKey);
            tokenModifierMappings.delete(folderKey);

            for (const document of workspace.textDocuments) {
                const documentFolder = workspace.getWorkspaceFolder(document.uri);
                if (!documentFolder) {
                    continue;
                }

                const outerMost = getOuterMostWorkspaceFolder(documentFolder);
                if (getWorkspaceFolderKey(outerMost) === folderKey) {
                    diagnosticCollection.delete(document.uri);
                }
            }
        }

        workspace.textDocuments.forEach(didOpenTextDocument);
    });
}

export async function deactivate(): Promise<void> {
    await Promise.all(
        [...clients.values()].map((client) => client.stop().catch(console.error))
    );
}

function handleDiagnostics(
    client: LanguageClient,
    diagnosticCollection: vscode.DiagnosticCollection,
    params: any
) {
    try {
        const {uri, diagnostics} = params;

        // Log for debugging (only in development)
        if (process.env.NODE_ENV === 'development') {
            console.log(`Received ${diagnostics.length} diagnostics for ${uri}`);
        }

        // Convert the URI string to a VS Code URI
        const docUri = client.protocol2CodeConverter.asUri(uri);

        // Convert LSP diagnostics to VS Code diagnostics
        const vscDiagnostics = diagnostics.map((diag: any) => {
            // Convert range from LSP to VS Code format
            const range = client.protocol2CodeConverter.asRange(diag.range) ||
                new vscode.Range(0, 0, 0, 0);

            // Convert severity level
            let severity: vscode.DiagnosticSeverity = vscode.DiagnosticSeverity.Error;
            if (diag.severity !== undefined) {
                const convertedSeverity = client.protocol2CodeConverter.asDiagnosticSeverity(diag.severity);
                if (convertedSeverity !== undefined) {
                    severity = convertedSeverity as vscode.DiagnosticSeverity;
                }
            }

            const diagnostic = new vscode.Diagnostic(
                range,
                diag.message,
                severity
            );

            // Set additional properties if available
            if (diag.code !== undefined) {
                if (typeof diag.code === 'string' || typeof diag.code === 'number') {
                    diagnostic.code = diag.code;
                } else if (diag.code && typeof diag.code === 'object') {
                    const targetUri = diag.code.target ? client.protocol2CodeConverter.asUri(diag.code.target) : undefined;
                    if (targetUri) {
                        diagnostic.code = {
                            value: diag.code.value,
                            target: targetUri
                        };
                    } else {
                        diagnostic.code = diag.code.value;
                    }
                }
            }

            if (diag.source) {
                diagnostic.source = diag.source;
            }

            if (diag.tags && Array.isArray(diag.tags)) {
                diagnostic.tags = diag.tags
                    .map((tag: number) => {
                        switch (tag) {
                            case 1:
                                return vscode.DiagnosticTag.Unnecessary;
                            case 2:
                                return vscode.DiagnosticTag.Deprecated;
                            default:
                                return undefined;
                        }
                    })
                    .filter((tag: vscode.DiagnosticTag | undefined): tag is vscode.DiagnosticTag => tag !== undefined);
            }

            if (diag.relatedInformation && Array.isArray(diag.relatedInformation)) {
                diagnostic.relatedInformation = diag.relatedInformation.map((info: any) => {
                    const location = client.protocol2CodeConverter.asLocation(info.location);
                    return new vscode.DiagnosticRelatedInformation(location, info.message);
                });
            }

            return diagnostic;
        });

        // Set diagnostics for the document (pass version if available for better performance)
        diagnosticCollection.set(docUri, vscDiagnostics);

        if (process.env.NODE_ENV === 'development') {
            console.log(`Set ${vscDiagnostics.length} diagnostics for ${docUri.toString()}`);
        }
    } catch (error) {
        console.error('Error processing diagnostics:', error);
        // Don't rethrow - we don't want to crash the extension
    }
}

/**
 * Convert LSP document symbols to VS Code format
 */
function convertDocumentSymbols(
    symbols: any[],
    client: LanguageClient,
    document: vscode.TextDocument
): vscode.DocumentSymbol[] {
    const folder = workspace.getWorkspaceFolder(document.uri);
    let mapping: Map<number, vscode.SymbolKind> | undefined;
    if (folder) {
        const outerMost = getOuterMostWorkspaceFolder(folder);
        const folderKey = getWorkspaceFolderKey(outerMost);
        mapping = symbolKindMappings.get(folderKey);
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

/**
 * Convert LSP completion item to VS Code completion item
 */
export function convertCompletionItem(item: any): vscode.CompletionItem {
    const completionItem = new vscode.CompletionItem(
        item.label,
        item.kind ? convertCompletionItemKind(item.kind) : vscode.CompletionItemKind.Text
    );

    if (item.detail) {
        completionItem.detail = item.detail;
    }
    if (item.documentation) {
        if (typeof item.documentation === 'string') {
            completionItem.documentation = new vscode.MarkdownString(item.documentation);
        } else {
            completionItem.documentation = new vscode.MarkdownString(item.documentation.value);
        }
    }
    if (item.sortText) {
        completionItem.sortText = item.sortText;
    }
    if (item.filterText) {
        completionItem.filterText = item.filterText;
    }
    if (item.insertText) {
        completionItem.insertText = item.insertText;
    }
    if (item.textEdit) {
        const range = new vscode.Range(
            item.textEdit.range.start.line,
            item.textEdit.range.start.character,
            item.textEdit.range.end.line,
            item.textEdit.range.end.character
        );
        completionItem.textEdit = new vscode.TextEdit(range, item.textEdit.newText);
    }

    return completionItem;
}

/**
 * Convert LSP completion item kind to VS Code kind
 */
export function convertCompletionItemKind(kind: number): vscode.CompletionItemKind {
    const kindMap: { [key: number]: vscode.CompletionItemKind } = {
        1: vscode.CompletionItemKind.Text,
        2: vscode.CompletionItemKind.Method,
        3: vscode.CompletionItemKind.Function,
        4: vscode.CompletionItemKind.Constructor,
        5: vscode.CompletionItemKind.Field,
        6: vscode.CompletionItemKind.Variable,
        7: vscode.CompletionItemKind.Class,
        8: vscode.CompletionItemKind.Interface,
        9: vscode.CompletionItemKind.Module,
        10: vscode.CompletionItemKind.Property,
        11: vscode.CompletionItemKind.Unit,
        12: vscode.CompletionItemKind.Value,
        13: vscode.CompletionItemKind.Enum,
        14: vscode.CompletionItemKind.Keyword,
        15: vscode.CompletionItemKind.Snippet,
        16: vscode.CompletionItemKind.Color,
        17: vscode.CompletionItemKind.File,
        18: vscode.CompletionItemKind.Reference,
        19: vscode.CompletionItemKind.Folder,
        20: vscode.CompletionItemKind.EnumMember,
        21: vscode.CompletionItemKind.Constant,
        22: vscode.CompletionItemKind.Struct,
        23: vscode.CompletionItemKind.Event,
        24: vscode.CompletionItemKind.Operator,
        25: vscode.CompletionItemKind.TypeParameter,
    };

    return kindMap[kind] || vscode.CompletionItemKind.Text;
}

/**
 * Register language feature providers for diagnostics, completion, hover, etc.
 */
function registerLanguageFeatures(context: ExtensionContext) {
    console.log('Registering language features...');

    // Register completion item provider
    context.subscriptions.push(
        languages.registerCompletionItemProvider(
            [
                {scheme: 'file', language: 'game-script'},
                {scheme: 'file', language: 'soup'}
            ],
            {
                provideCompletionItems: async (document, position, token) => {
                    try {
                        console.log(`Completion requested at ${document.uri.toString()}:${position.line}:${position.character}`);

                        const client = getClientForDocument(document);
                        if (!client || !client.isRunning()) {
                            return [];
                        }

                        const params = {
                            textDocument: {uri: document.uri.toString()},
                            position: {line: position.line, character: position.character}
                        };

                        const response = await client.sendRequest<any>(
                            'textDocument/completion',
                            params,
                            token
                        );

                        console.log('Completion response:', response);

                        if (!response) {
                            return [];
                        }

                        const items = response.items && Array.isArray(response.items)
                            ? response.items
                            : Array.isArray(response)
                                ? response
                                : [];

                        return items.map((item: any) => {
                            const completionItem = convertCompletionItem(item);
                            (completionItem as any).data = {
                                ...((completionItem as any).data && typeof (completionItem as any).data === 'object' ? (completionItem as any).data : {}),
                                __documentUri: document.uri.toString()
                            };
                            return completionItem;
                        });
                    } catch (error) {
                        console.error('Error in completion provider:', error);
                        return [];
                    }
                },
                resolveCompletionItem: async (item, token) => {
                    try {
                        const documentUri = (item as any).data?.__documentUri || window.activeTextEditor?.document.uri.toString();
                        if (!documentUri) {
                            return item;
                        }

                        const doc = workspace.textDocuments.find(d => d.uri.toString() === documentUri);
                        const client = doc ? getClientForDocument(doc) : undefined;
                        if (!client || !client.isRunning()) {
                            return item;
                        }

                        console.log('Resolving completion item:', item);
                        const response = await client.sendRequest<any>(
                            'completionItem/resolve',
                            item,
                            token
                        );
                        console.log('Resolved completion item:', response);
                        return response || item;
                    } catch (error) {
                        console.error('Error resolving completion item:', error);
                        return item;
                    }
                }
            },
            '.' // Trigger completion on dot
        )
    );

    // Register hover provider
    context.subscriptions.push(
        languages.registerHoverProvider(
            [
                {scheme: 'file', language: 'game-script'},
                {scheme: 'file', language: 'soup'}
            ],
            {
                provideHover: async (document, position, token) => {
                    try {
                        console.log(`Hover requested at ${document.uri.toString()}:${position.line}:${position.character}`);

                        const client = getClientForDocument(document);
                        if (!client || !client.isRunning()) {
                            return null;
                        }

                        const params = {
                            textDocument: {uri: document.uri.toString()},
                            position: {line: position.line, character: position.character}
                        };

                        const response = await client.sendRequest<any>(
                            'textDocument/hover',
                            params,
                            token
                        );

                        console.log('Hover response:', response);

                        if (!response) {
                            return null;
                        }

                        let contents: any[] = [];
                        if (response.contents) {
                            if (Array.isArray(response.contents)) {
                                contents = response.contents;
                            } else {
                                contents = [response.contents];
                            }
                        }

                        return {
                            contents: contents,
                            range: response.range ? client.protocol2CodeConverter.asRange(response.range) : undefined
                        };
                    } catch (error) {
                        console.error('Error in hover provider:', error);
                        return null;
                    }
                }
            }
        )
    );

    // Register document symbol provider (for outline/symbolizing)
    context.subscriptions.push(
        languages.registerDocumentSymbolProvider(
            [
                {scheme: 'file', language: 'game-script'},
                {scheme: 'file', language: 'soup'}
            ],
            {
                provideDocumentSymbols: async (document, token) => {
                    try {
                        console.log(`Document symbols requested for ${document.uri.toString()}`);

                        const client = getClientForDocument(document);
                        if (!client || !client.isRunning()) {
                            return [];
                        }

                        const response = await client.sendRequest<any>(
                            'textDocument/documentSymbol',
                            {
                                textDocument: {uri: document.uri.toString()}
                            },
                            token
                        );

                        console.log('Document symbols response:', response);

                        if (!response) {
                            return [];
                        }

                        return convertDocumentSymbols(response, client, document);
                    } catch (error) {
                        console.error('Error in document symbol provider:', error);
                        return [];
                    }
                }
            }
        )
    );

    // Register folding range provider
    context.subscriptions.push(
        languages.registerFoldingRangeProvider(
            [
                {scheme: 'file', language: 'game-script'},
                {scheme: 'file', language: 'soup'}
            ],
            {
                provideFoldingRanges: async (document, context, token) => {
                    try {
                        console.log(`Folding ranges requested for ${document.uri.toString()}`);

                        const client = getClientForDocument(document);
                        if (!client || !client.isRunning()) {
                            return [];
                        }

                        const response = await client.sendRequest<any>(
                            'textDocument/foldingRange',
                            {
                                textDocument: {uri: document.uri.toString()}
                            },
                            token
                        );

                        console.log('Folding ranges response:', response);

                        if (!response) {
                            return [];
                        }

                        // Convert LSP folding ranges to VS Code format
                        return response.map((range: any) => {
                            const startLine = range.startLine || 0;
                            const endLine = range.endLine || 0;
                            const kind = range.kind as vscode.FoldingRangeKind | undefined;

                            return new vscode.FoldingRange(
                                startLine,
                                endLine,
                                kind
                            );
                        });
                    } catch (error) {
                        console.error('Error in folding range provider:', error);
                        return [];
                    }
                }
            }
        )
    );

    // Register document semantic tokens provider
    context.subscriptions.push(
        languages.registerDocumentSemanticTokensProvider(
            [
                {scheme: 'file', language: 'game-script'},
                {scheme: 'file', language: 'soup'}
            ],
            {
                provideDocumentSemanticTokens: async (document, token) => {
                    try {
                        console.log(`Semantic tokens requested for ${document.uri.toString()}`);

                        const client = getClientForDocument(document);
                        if (!client || !client.isRunning()) {
                            return null;
                        }

                        const folder = workspace.getWorkspaceFolder(document.uri);
                        if (!folder) {
                            return null;
                        }

                        const outerMost = getOuterMostWorkspaceFolder(folder);
                        const folderKey = getWorkspaceFolderKey(outerMost);

                        const typeMapping = tokenTypeMappings.get(folderKey);
                        const modifierMapping = tokenModifierMappings.get(folderKey);

                        if (!typeMapping || !modifierMapping) {
                            return null;
                        }

                        const response = await client.sendRequest<any>(
                            'textDocument/semanticTokens/full',
                            {
                                textDocument: {uri: document.uri.toString()}
                            },
                            token
                        );

                        console.log('Semantic tokens response:', response);

                        if (!response || !response.data) {
                            return null;
                        }

                        const builder = new vscode.SemanticTokensBuilder(semanticTokensLegend);

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
                                    if (mapped !== undefined) {
                                        mappedMods |= (1 << mapped);
                                    }
                                }
                            }

                            builder.push(deltaLine, deltaStart, length, mappedType, mappedMods);
                        }

                        return builder.build();
                    } catch (error) {
                        console.error('Error in semantic tokens provider:', error);
                        return null;
                    }
                }
            },
            semanticTokensLegend
        )
    );
}

