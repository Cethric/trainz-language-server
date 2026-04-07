import * as assert from 'assert';
import * as vscode from 'vscode';
import * as path from 'path';

suite('Extension Test Suite', () => {
	vscode.window.showInformationMessage('Start all tests.');

	test('Extension should be present', () => {
		assert.ok(vscode.extensions.getExtension('gs-lsp.gs-lsp-vscode'));
	});

	test('Should activate extension', async () => {
		const ext = vscode.extensions.getExtension('gs-lsp.gs-lsp-vscode');
		await ext?.activate();
		assert.strictEqual(ext?.isActive, true);
	});

	test('Should identify GS files', async () => {
		const uri = vscode.Uri.file(path.join(vscode.workspace.rootPath || '', 'test.gs'));
		const doc = await vscode.workspace.openTextDocument(uri.with({ scheme: 'untitled' }));
		assert.strictEqual(doc.languageId, 'game-script');
	});

	test('Should identify Soup files', async () => {
		const uri = vscode.Uri.file(path.join(vscode.workspace.rootPath || '', 'test.txt'));
		const doc = await vscode.workspace.openTextDocument(uri.with({ scheme: 'untitled' }));
		assert.strictEqual(doc.languageId, 'soup');
	});

	test('Should provide folding ranges for GS', async function() {
		this.timeout(20000); // Increase timeout for LSP startup
		
		const serverPath = '/Users/rogan/Developer/sources/gs-lsp/target/release/gs-lsp';
		const config = vscode.workspace.getConfiguration('gs-lsp');
		await config.update('serverPath', serverPath, vscode.ConfigurationTarget.Global);

		const content = 'class Test {\n  void foo() {\n  }\n};';
		const doc = await vscode.workspace.openTextDocument({ content, language: 'game-script' });
		await vscode.window.showTextDocument(doc);
		
		// Wait a bit for LSP to start and provide ranges
		await new Promise(resolve => setTimeout(resolve, 5000));
		
		const ranges = await vscode.commands.executeCommand<vscode.FoldingRange[]>(
			'vscode.executeFoldingRangeProvider',
			doc.uri
		);
		
		assert.ok(ranges && ranges.length > 0, 'Should have folding ranges');
	});

	test('Should provide semantic tokens for GS', async function() {
		this.timeout(60000);
		
		const content = 'class Test {\n  public void Run() {\n    string s = "Hello";\n  }\n};';
		const uri = vscode.Uri.file(path.join(vscode.workspace.rootPath || '/tmp', 'test_tokens.gs'));
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
		console.log(`Folding ranges for GS: ${ranges?.length}`);
		assert.ok(ranges && ranges.length > 0, 'Should have folding ranges for GS before tokens');
		
		// Wait a bit more
		await new Promise(resolve => setTimeout(resolve, 5000));

		const tokens = await vscode.commands.executeCommand<vscode.SemanticTokens>(
			'vscode.provideDocumentSemanticTokens',
			doc.uri
		);

		console.log(`Semantic tokens for GS: ${tokens ? JSON.stringify(tokens).substring(0, 100) : 'undefined'}`);

		// Clean up
		if (fs.existsSync(uri.fsPath)) { fs.unlinkSync(uri.fsPath); }

		assert.ok(tokens && tokens.data && tokens.data.length > 0, 'Should have semantic tokens');
		assert.strictEqual(tokens.data.length % 5, 0, 'Tokens data length should be multiple of 5');
	});

	test('Should provide semantic tokens for Soup', async function() {
		this.timeout(60000);
		
		const content = `kind "train"\nkuid <kuid:1234:5678>\nusername "Sample Train"\ndescription "A sample train for testing"\n\nextensions {\n    author "Junie"\n    version 1.0\n}\n\nmesh-table {\n    default {\n        mesh "body.lm"\n        auto-create 1\n    }\n}`;
		const uri = vscode.Uri.file(path.join(vscode.workspace.rootPath || '/tmp', 'test_tokens.txt'));
		const fs = require('fs');
		fs.writeFileSync(uri.fsPath, content);

		const doc = await vscode.workspace.openTextDocument(uri);
		await vscode.window.showTextDocument(doc);
		
		// Wait for LSP
		await new Promise(resolve => setTimeout(resolve, 15000));
		
		const tokens = await vscode.commands.executeCommand<vscode.SemanticTokens>(
			'vscode.provideDocumentSemanticTokens',
			doc.uri
		);
		console.log(`Semantic tokens for Soup: ${tokens ? JSON.stringify(tokens).substring(0, 100) : 'undefined'}`);
		
		// Clean up
		if (fs.existsSync(uri.fsPath)) { fs.unlinkSync(uri.fsPath); }

		assert.ok(tokens && tokens.data && tokens.data.length > 0, 'Should have semantic tokens for Soup');
		assert.strictEqual(tokens.data.length % 5, 0, 'Tokens data length should be multiple of 5');
	});
});
