import * as assert from 'assert';
import * as vscode from 'vscode';
import * as path from 'path';
import { gsSemanticTokensProvider } from '../../features/gs/semanticTokens';
import { acsSemanticTokensProvider } from '../../features/acs/semanticTokens';
import { getClientForDocument } from '../../utils';
import { LanguageClient } from 'vscode-languageclient/node';

suite('E2E Test Suite', () => {
    suiteSetup(async function () {
        this.timeout(120000);

        const ext = vscode.extensions.getExtension('cethric.trainz-language-server');
        if (ext) {
            await ext.activate();
        } else {
            console.log('[Trainz LSP] Extension not found!');
        }

        const binPath = path.resolve(__dirname, '../../../../../target/release/trainz-language-server');
        await vscode.workspace.getConfiguration('trainz-language-server').update('server-bin', binPath, vscode.ConfigurationTarget.Global);
        await vscode.workspace.getConfiguration('trainz-language-server').update('trace.server', 'verbose', vscode.ConfigurationTarget.Global);
        // Give VS Code time to propagate the config change before the client starts
        await new Promise(resolve => setTimeout(resolve, 1000));

        // Open ACS fixture first to trigger client startup, wait for it to initialize,
        // then open the GS fixture so the client is already running and can process it.
        const gsFixture = path.resolve(__dirname, '../../../src/test/fixtures/test.gs');
        const acsFixture = path.resolve(__dirname, '../../../src/test/fixtures/config.txt');

        const acsDoc = await vscode.workspace.openTextDocument(vscode.Uri.file(acsFixture));
        await vscode.window.showTextDocument(acsDoc);

        // Wait for the language server to complete the LSP initialize handshake before opening GS
        console.log('[Trainz LSP] Waiting for language server to initialize...');
        await new Promise(resolve => setTimeout(resolve, 10000));
        console.log('[Trainz LSP] Language server ready, opening GS fixture...');

        const gsDoc = await vscode.workspace.openTextDocument(vscode.Uri.file(gsFixture));
        await vscode.window.showTextDocument(gsDoc);

        // Wait for the GS file to be indexed
        await new Promise(resolve => setTimeout(resolve, 5000));
        console.log('[Trainz LSP] GS fixture opened and indexed.');
    });

    test('Extension should load and activate for GS files', async () => {
        const fixturePath = path.resolve(__dirname, '../../../src/test/fixtures/test.gs');
        const uri = vscode.Uri.file(fixturePath);
        const document = await vscode.workspace.openTextDocument(uri);
        await vscode.window.showTextDocument(document);

        assert.strictEqual(document.languageId, 'game-script');
    }).timeout(10000);

    test('Extension should load and activate for ACS files', async () => {
        const fixturePath = path.resolve(__dirname, '../../../src/test/fixtures/config.txt');
        const uri = vscode.Uri.file(fixturePath);
        const document = await vscode.workspace.openTextDocument(uri);
        await vscode.window.showTextDocument(document);

        assert.strictEqual(document.languageId, 'acs');
    }).timeout(10000);

    test('Semantic tokens should work for GS files', async () => {
        const fixturePath = path.resolve(__dirname, '../../../src/test/fixtures/test.gs');
        const uri = vscode.Uri.file(fixturePath);
        const document = await vscode.workspace.openTextDocument(uri);
        await vscode.window.showTextDocument(document);
        console.log('[Trainz LSP] GS doc languageId:', document.languageId);

        // Get the live clients map via the cached require of the bundled extension
        // (same module instance that VS Code's extension host loaded)
        // eslint-disable-next-line @typescript-eslint/no-require-imports
        const extModule = require(path.resolve(__dirname, '../../../dist/extension.js')) as { getClients: () => Map<string, LanguageClient> };
        const liveClients = extModule.getClients();
        console.log('[Trainz LSP] GS liveClients size:', liveClients?.size);

        const cancelSource = new vscode.CancellationTokenSource();
        let tokens: vscode.SemanticTokens | null | undefined;
        for (let i = 0; i < 20; i++) {
            const client = getClientForDocument(document, liveClients);
            console.log(`[Trainz LSP] GS tokens attempt ${i}: client=${client ? 'found' : 'not found'}`);
            if (client) {
                tokens = await gsSemanticTokensProvider.provideDocumentSemanticTokens(document, cancelSource.token, client);
                console.log(`[Trainz LSP] GS tokens attempt ${i} result:`, tokens ? `data.length=${tokens.data.length}` : 'null');
                if (tokens && tokens.data.length > 0) break;
            }
            await new Promise(resolve => setTimeout(resolve, 3000));
        }
        cancelSource.dispose();
        assert.ok(tokens && tokens.data.length > 0, 'Semantic tokens should be returned for GS file');
    }).timeout(90000);

    test('Semantic tokens should work for ACS files', async () => {
        const fixturePath = path.resolve(__dirname, '../../../src/test/fixtures/config.txt');
        const uri = vscode.Uri.file(fixturePath);
        const document = await vscode.workspace.openTextDocument(uri);
        await vscode.window.showTextDocument(document);
        console.log('[Trainz LSP] ACS doc languageId:', document.languageId);

        // Get the live clients map via the cached require of the bundled extension
        // (same module instance that VS Code's extension host loaded)
        // eslint-disable-next-line @typescript-eslint/no-require-imports
        const extModule = require(path.resolve(__dirname, '../../../dist/extension.js')) as { getClients: () => Map<string, LanguageClient> };
        const liveClients = extModule.getClients();
        console.log('[Trainz LSP] ACS liveClients size:', liveClients?.size);

        const cancelSource = new vscode.CancellationTokenSource();
        let tokens: vscode.SemanticTokens | null | undefined;
        for (let i = 0; i < 20; i++) {
            const client = getClientForDocument(document, liveClients);
            console.log(`[Trainz LSP] ACS tokens attempt ${i}: client=${client ? 'found' : 'not found'}`);
            if (client) {
                tokens = await acsSemanticTokensProvider.provideDocumentSemanticTokens(document, cancelSource.token, client);
                console.log(`[Trainz LSP] ACS tokens attempt ${i} result:`, tokens ? `data.length=${tokens.data.length}` : 'null');
                if (tokens && tokens.data.length > 0) break;
            }
            await new Promise(resolve => setTimeout(resolve, 3000));
        }
        cancelSource.dispose();
        assert.ok(tokens && tokens.data.length > 0, 'Semantic tokens should be returned for ACS file');
    }).timeout(90000);
});
