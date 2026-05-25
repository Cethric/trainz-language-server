import * as vscode from 'vscode';
import {LanguageClient} from 'vscode-languageclient/node';
import * as utils from '../../utils';

const gsLegend = new vscode.SemanticTokensLegend(
    ['namespace', 'type', 'class', 'enum', 'interface', 'struct', 'typeParameter', 'parameter', 'variable', 'property', 'enumMember', 'event', 'function', 'method', 'macro', 'keyword', 'modifier', 'comment', 'string', 'number', 'regexp', 'operator'],
    ['declaration', 'definition', 'readonly', 'static', 'deprecated', 'abstract', 'async', 'modification', 'documentation', 'defaultLibrary']
);

export function parseGsSemanticTokensData(data: any[], builder: any) {
    for (let i = 0; i < data.length; i += 5) {
        builder.push(
            data[i],
            data[i + 1],
            data[i + 2],
            data[i + 3],
            data[i + 4]
        );
    }
}

export const gsSemanticTokensProvider = {
    provideDocumentSemanticTokens: async (document: vscode.TextDocument, token: vscode.CancellationToken, client: LanguageClient) => {
        console.log('[Trainz LSP] GS provideDocumentSemanticTokens called for', document.uri.toString());
        const response = await client.sendRequest<any>('textDocument/semanticTokens/full', {
            textDocument: {uri: document.uri.toString()}
        }, token);
        console.log('[Trainz LSP] GS semantic tokens response:', response ? `data length=${response.data?.length}` : 'null');
        if (!response || !response.data) return null;

        const builder = new vscode.SemanticTokensBuilder(gsLegend);
        parseGsSemanticTokensData(response.data, builder);
        return builder.build();
    }
};

export function registerGsSemanticTokens(
    context: vscode.ExtensionContext,
    clients: Map<string, LanguageClient>
) {
    context.subscriptions.push(
        vscode.languages.registerDocumentSemanticTokensProvider(
            [{scheme: 'file', language: 'game-script'}],
            {
                provideDocumentSemanticTokens: (document, token) => {
                    const client = utils.getClientForDocument(document, clients);
                    if (!client) return null;
                    return gsSemanticTokensProvider.provideDocumentSemanticTokens(document, token, client);
                }
            },
            gsLegend
        )
    );
}
