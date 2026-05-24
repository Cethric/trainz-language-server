import * as assert from 'assert';
import * as vscode from 'vscode';
import { parseGsHoverResponse } from '../../features/gs/hover';

suite('GS Hover Test Suite', () => {
    test('parseGsHoverResponse parses correctly', () => {
        const response = { contents: 'hover content' };
        const result = parseGsHoverResponse(response);

        assert.ok(result instanceof vscode.Hover);
        assert.strictEqual((result as vscode.Hover).contents[0], 'hover content');
    });

    test('parseGsHoverResponse handles null response', () => {
        const result = parseGsHoverResponse(null);
        assert.strictEqual(result, null);
    });
});
