import * as assert from 'assert';
import { parseAcsSemanticTokensData } from '../../features/acs/semanticTokens';

suite('ACS Semantic Tokens Test Suite', () => {
    test('parseAcsSemanticTokensData parses correctly', () => {
        const builder = {
            tokens: [] as any[],
            push(line: number, char: number, len: number, type: number, mod: number) {
                this.tokens.push({ line, char, len, type, mod });
            }
        };

        parseAcsSemanticTokensData([0, 0, 5, 0, 0, 1, 5, 3, 1, 0], builder);

        assert.strictEqual(builder.tokens.length, 2);
        assert.deepStrictEqual(builder.tokens[0], { line: 0, char: 0, len: 5, type: 0, mod: 0 });
        assert.deepStrictEqual(builder.tokens[1], { line: 1, char: 5, len: 3, type: 1, mod: 0 });
    });
});
