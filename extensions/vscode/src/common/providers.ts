import * as vscode from 'vscode';
import {LanguageClient} from 'vscode-languageclient/node';
import * as utils from '../utils';

export function registerProviders(
    context: vscode.ExtensionContext,
    languageId: string,
    clients: Map<string, LanguageClient>,
    diagnosticCollection: vscode.DiagnosticCollection
) {
    const selector = [{scheme: 'file', language: languageId}];

    // Register completion item provider
    // ...
}
