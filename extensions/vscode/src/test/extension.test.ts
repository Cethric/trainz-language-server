import * as assert from 'assert';
import * as vscode from 'vscode';
import * as path from 'node:path';
import * as os from 'node:os';
import { randomUUID } from 'node:crypto';
import * as fs from 'node:fs';

let suiteRootPath: string;

suite('Extension Test Suite', () => {
    vscode.window.showInformationMessage('Start all tests.');

    suiteSetup(async () => {
        suiteRootPath = path.join(os.tmpdir(), 'gs-lsp-test-' + randomUUID());
        fs.mkdirSync(suiteRootPath, { recursive: true });
        
        // Replace all existing workspace folders with the new suiteRootPath
        await vscode.workspace.updateWorkspaceFolders(
            0,
            vscode.workspace.workspaceFolders?.length || 0,
            { uri: vscode.Uri.file(suiteRootPath) }
        );

        // Configure server-bin for all tests
        const serverPath = '/Users/rogan/Developer/sources/gs-lsp/target/release/trainz-language-server';
        const validationPath = '/Users/rogan/Developer/sources/gs-lsp/crates/acs-text-validators/tests/data';
        const logFile = path.join(suiteRootPath, 'server.log');
        const config = vscode.workspace.getConfiguration('trainz-language-server');
        await config.update('server-bin', serverPath, vscode.ConfigurationTarget.Global);
        await config.update('validation-path', validationPath, vscode.ConfigurationTarget.Global);
        await config.update('log-file', logFile, vscode.ConfigurationTarget.Global);
        await config.update('log-level', 'debug', vscode.ConfigurationTarget.Global);
    });

    test('Extension should be present', () => {
        vscode.window.showInformationMessage(`Extensions: ${vscode.extensions.all.map(e => e.id).filter(x => x.includes("trainz")).join(" ; ")}`);
        assert.ok(vscode.extensions.getExtension('cethric.trainz-language-server'));
    });

    test('Should activate extension', async () => {
        const ext = vscode.extensions.getExtension('cethric.trainz-language-server');
        await ext?.activate();
        assert.strictEqual(ext?.isActive, true);
    });

    test('Should identify GS files', async () => {
        const uri = vscode.Uri.file(path.join(suiteRootPath, 'test.gs'));
        const doc = await vscode.workspace.openTextDocument(uri.with({scheme: 'untitled'}));
        assert.strictEqual(doc.languageId, 'game-script');
    });

    test('Should identify AcsText files', async () => {
        const uri = vscode.Uri.file(path.join(suiteRootPath, 'config.txt'));
        const doc = await vscode.workspace.openTextDocument(uri.with({scheme: 'untitled'}));
        assert.strictEqual(doc.languageId, 'acs');
    });

    test('Should provide folding ranges for GS', async function () {
        this.timeout(20000); // Increase timeout for LSP startup

        const content = 'class Test {\n  void foo() {\n  }\n};';
        const doc = await vscode.workspace.openTextDocument({content, language: 'game-script'});
        await vscode.window.showTextDocument(doc);

        // Wait a bit for LSP to start and provide ranges
        await new Promise(resolve => setTimeout(resolve, 5000));

        const ranges = await vscode.commands.executeCommand<vscode.FoldingRange[]>(
            'vscode.executeFoldingRangeProvider',
            doc.uri
        );

        assert.ok(ranges && ranges.length > 0, 'Should have folding ranges');
    });

    test('Should provide semantic tokens for GS', async function () {
        this.timeout(60000);

        const content = 'class Test {\n  public void Run() {\n    string s = "Hello";\n  }\n};';
        const rootPath = suiteRootPath;
        const uri = vscode.Uri.file(path.join(rootPath, 'test_tokens.gs'));

        const fs = require('fs');
        fs.writeFileSync(uri.fsPath, content);

        const doc = await vscode.workspace.openTextDocument(uri);
        await vscode.window.showTextDocument(doc);

        // Wait longer for LSP to process the file and provide ranges first as a check
        await new Promise(resolve => setTimeout(resolve, 15000));

        const ranges = await vscode.commands.executeCommand<vscode.FoldingRange[]>(
            'vscode.executeFoldingRangeProvider',
            doc.uri
        );
        assert.ok(ranges && ranges.length > 0, 'Should have folding ranges for GS before tokens');

        // Wait a bit more
        await new Promise(resolve => setTimeout(resolve, 5000));

        const tokens = await vscode.commands.executeCommand<vscode.SemanticTokens>(
            'vscode.provideDocumentSemanticTokens',
            doc.uri
        );

        // Clean up
        if (fs.existsSync(uri.fsPath)) {
            fs.unlinkSync(uri.fsPath);
        }

        assert.ok(tokens && tokens.data && tokens.data.length > 0, 'Should have semantic tokens');
        assert.strictEqual(tokens.data.length % 5, 0, 'Tokens data length should be multiple of 5');
    });

    test('Should provide semantic tokens for AcsText', async function () {
        this.timeout(60000);

        const content = `levels
{
  icon levels
  kind "Structure"
}

thumbnails
{
	icon thumbnails
	menu-token "$ccp_thumbnails_menu-name"
	description "" 
	unique 1
	default-amount 1
	kind "structure"
  array-element
  {
    kind-value-A "thumbnails-element"
    kind-value-B "conditions"
  }
  validation
  {
    UniqueNames
  } 
}
`;
        const rootPath = suiteRootPath;
        const uri = vscode.Uri.file(path.join(rootPath, 'config.txt'));
        
        const fs = require('fs');
        fs.writeFileSync(uri.fsPath, content);
        
        const doc = await vscode.workspace.openTextDocument(uri);
        await vscode.window.showTextDocument(doc);

        // Wait for the language ID to be set to 'acs'
        let retries = 0;
        while (doc.languageId !== 'acs' && retries < 10) {
            await new Promise(resolve => setTimeout(resolve, 1000));
            retries++;
        }
        
        // Wait for LSP
        await new Promise(resolve => setTimeout(resolve, 15000));

        const tokens = await vscode.commands.executeCommand<vscode.SemanticTokens>(
            'vscode.provideDocumentSemanticTokens',
            doc.uri
        );

        // Clean up
        if (fs.existsSync(uri.fsPath)) {
            fs.unlinkSync(uri.fsPath);
        }

        assert.ok(tokens, 'Should have semantic tokens object for AcsText');
    });
});
