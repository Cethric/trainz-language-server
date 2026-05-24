import * as assert from 'assert';
import * as vscode from 'vscode';
import { parseAcsCompletionResponse } from '../../features/acs/completion';

suite('ACS Completion Test Suite', () => {
    test('parseAcsCompletionResponse parses correctly', () => {
        const response = {
            items: [
                { label: 'item1' },
                { label: 'item2' }
            ]
        };

        const result = parseAcsCompletionResponse(response);

        assert.strictEqual(result.length, 2);
        assert.strictEqual(result[0].label, 'item1');
        assert.strictEqual(result[1].label, 'item2');
    });

    test('parseAcsCompletionResponse handles empty response', () => {
        const result = parseAcsCompletionResponse(null);
        assert.strictEqual(result.length, 0);
    });
});
