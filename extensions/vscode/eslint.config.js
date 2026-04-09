import js from "@eslint/js";
import tseslint from "@typescript-eslint/eslint-plugin";
import tsparser from "@typescript-eslint/parser";
import globals from "globals";

export default [
    js.configs.recommended,
    {
        files: ["src/**/*.ts"],
        languageOptions: {
            parser: tsparser,
            parserOptions: {
                ecmaVersion: 6,
                sourceType: "module"
            },
            globals: {
                ...globals.node,
                ...globals.browser,
                Thenable: "readonly"
            }
        },
        plugins: {
            "@typescript-eslint": tseslint
        },
        rules: {
            "@typescript-eslint/naming-convention": ["warn", {
                selector: "variableLike",
                format: ["camelCase", "UPPER_CASE"],
                filter: {
                    regex: "^(TRAINZ_LSP_|RUST_LOG)$",
                    match: false
                },
                leadingUnderscore: "allow"
            }],
            "curly": "warn",
            "eqeqeq": "warn",
            "no-throw-literal": "warn",
            "semi": "off",
            "no-unused-vars": ["error", { "argsIgnorePattern": "^_", "varsIgnorePattern": "^_" }]
        }
    },
    {
        files: ["src/test/**/*.ts"],
        languageOptions: {
            parser: tsparser,
            parserOptions: {
                ecmaVersion: 6,
                sourceType: "module"
            },
            globals: {
                ...globals.node,
                ...globals.mocha
            }
        },
        plugins: {
            "@typescript-eslint": tseslint
        },
        rules: {
            "@typescript-eslint/naming-convention": ["warn", {
                selector: "variableLike",
                format: ["camelCase", "UPPER_CASE"],
                filter: {
                    regex: "^(TRAINZ_LSP_|RUST_LOG)$",
                    match: false
                },
                leadingUnderscore: "allow"
            }],
            "curly": "warn",
            "eqeqeq": "warn",
            "no-throw-literal": "warn",
            "semi": "off",
            "no-unused-vars": ["error", { "argsIgnorePattern": "^_", "varsIgnorePattern": "^_" }]
        }
    },
    {
        ignores: ["out/**", "dist/**", "**/*.d.ts"]
    }
];