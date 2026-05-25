import * as assert from 'assert';
import {parseGsDefinitionResponse} from '../../features/gs/definition';
import {LanguageClient} from 'vscode-languageclient/node';

suite('GS Definition Test Suite', () => {
    test('parseGsDefinitionResponse parses correctly', () => {
        const mockClient = {
            protocol2CodeConverter: {
                asDefinitionResult: (resp: any) => resp
            }
        } as any as LanguageClient;

        const response = {
            uri: 'file:///test.gs',
            range: {start: {line: 0, character: 0}, end: {line: 0, character: 5}}
        };
        const result = parseGsDefinitionResponse(mockClient, response);

        assert.deepStrictEqual(result, response);
    });

    test('parseGsDefinitionResponse handles null response', () => {
        const mockClient = {
            protocol2CodeConverter: {
                asDefinitionResult: (resp: any) => resp
            }
        } as any as LanguageClient;

        const result = parseGsDefinitionResponse(mockClient, null);
        assert.strictEqual(result, null);
    });
});
