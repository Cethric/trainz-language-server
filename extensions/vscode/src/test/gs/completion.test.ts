import * as assert from 'assert';
import * as vscode from 'vscode';
import { parseGsCompletionResponse } from '../../features/gs/completion';

suite('GS Completion Test Suite', () => {
    test('parseGsCompletionResponse parses correctly', () => {
        const response = {
            items: [
                { label: 'item1' },
                { label: 'item2' }
            ]
        };

        const result = parseGsCompletionResponse(response);

        assert.strictEqual(result.length, 2);
        assert.strictEqual(result[0].label, 'item1');
        assert.strictEqual(result[1].label, 'item2');
    });

    test('parseGsCompletionResponse handles empty response', () => {
        const result = parseGsCompletionResponse(null);
        assert.strictEqual(result.length, 0);
    });
});
