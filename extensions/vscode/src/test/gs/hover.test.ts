import * as assert from 'assert';
import * as vscode from 'vscode';
import {parseGsHoverResponse} from '../../features/gs/hover';

suite('GS Hover Test Suite', () => {
    test('parseGsHoverResponse parses MarkupContent correctly', () => {
        const response = {contents: {kind: 'markdown', value: 'hover content'}};
        const result = parseGsHoverResponse(response);

        assert.ok(result instanceof vscode.Hover);
        const content = (result as vscode.Hover).contents[0] as vscode.MarkdownString;
        assert.ok(content instanceof vscode.MarkdownString);
        assert.strictEqual(content.value, 'hover content');
    });

    test('parseGsHoverResponse sets isTrusted on MarkdownString', () => {
        const response = {contents: {kind: 'markdown', value: 'trusted content'}};
        const result = parseGsHoverResponse(response);

        assert.ok(result instanceof vscode.Hover);
        const content = (result as vscode.Hover).contents[0] as vscode.MarkdownString;
        assert.ok(content instanceof vscode.MarkdownString);
        assert.strictEqual(content.isTrusted, true);
    });

    test('parseGsHoverResponse renders multiline markdown content', () => {
        const multilineValue = '**Method:** `foo(bar: int)`\n\nParameter: `bar` — the bar value';
        const response = {contents: {kind: 'markdown', value: multilineValue}};
        const result = parseGsHoverResponse(response);

        assert.ok(result instanceof vscode.Hover);
        const content = (result as vscode.Hover).contents[0] as vscode.MarkdownString;
        assert.ok(content instanceof vscode.MarkdownString);
        assert.strictEqual(content.value, multilineValue);
    });

    test('parseGsHoverResponse handles MarkupContent with plaintext kind', () => {
        const response = {contents: {kind: 'plaintext', value: 'plain text hover'}};
        const result = parseGsHoverResponse(response);

        assert.ok(result instanceof vscode.Hover);
        const content = (result as vscode.Hover).contents[0] as vscode.MarkdownString;
        assert.ok(content instanceof vscode.MarkdownString);
        assert.strictEqual(content.value, 'plain text hover');
    });

    test('parseGsHoverResponse handles empty string value in MarkupContent', () => {
        const response = {contents: {kind: 'markdown', value: ''}};
        const result = parseGsHoverResponse(response);

        assert.ok(result instanceof vscode.Hover);
        const content = (result as vscode.Hover).contents[0] as vscode.MarkdownString;
        assert.ok(content instanceof vscode.MarkdownString);
        assert.strictEqual(content.value, '');
    });

    test('parseGsHoverResponse handles object contents without value property', () => {
        const contentsObj = {kind: 'markdown'};
        const response = {contents: contentsObj};
        const result = parseGsHoverResponse(response);

        assert.ok(result instanceof vscode.Hover);
        assert.strictEqual((result as vscode.Hover).contents[0], contentsObj);
    });

    test('parseGsHoverResponse falls back for plain string contents', () => {
        const response = {contents: 'hover content'};
        const result = parseGsHoverResponse(response);

        assert.ok(result instanceof vscode.Hover);
        assert.strictEqual((result as vscode.Hover).contents[0], 'hover content');
    });

    test('parseGsHoverResponse handles null response', () => {
        const result = parseGsHoverResponse(null);
        assert.strictEqual(result, null);
    });

    test('parseGsHoverResponse handles undefined response', () => {
        const result = parseGsHoverResponse(undefined);
        assert.strictEqual(result, null);
    });
});
