import * as vscode from 'vscode';

export class SoupEditorProvider implements vscode.CustomTextEditorProvider {

    public static register(context: vscode.ExtensionContext): vscode.Disposable {
        const provider = new SoupEditorProvider(context);
        const providerRegistration = vscode.window.registerCustomEditorProvider(SoupEditorProvider.viewType, provider);
        return providerRegistration;
    }

    private static readonly viewType = 'trainz-lsp.soupEditor';

    constructor(
        private readonly context: vscode.ExtensionContext
    ) { }

    public async resolveCustomTextEditor(
        document: vscode.TextDocument,
        webviewPanel: vscode.WebviewPanel,
        _token: vscode.CancellationToken
    ): Promise<void> {
        webviewPanel.webview.options = {
            enableScripts: true,
        };
        webviewPanel.webview.html = this.getHtmlForWebview(webviewPanel.webview);

        function updateWebview() {
            webviewPanel.webview.postMessage({
                type: 'update',
                text: document.getText(),
            });
        }

        const changeDocumentSubscription = vscode.workspace.onDidChangeTextDocument(e => {
            if (e.document.uri.toString() === document.uri.toString()) {
                updateWebview();
            }
        });

        webviewPanel.onDidDispose(() => {
            changeDocumentSubscription.dispose();
        });

        webviewPanel.webview.onDidReceiveMessage(e => {
            switch (e.type) {
                case 'change':
                    this.updateTextDocument(document, e.text);
                    return;
            }
        });

        updateWebview();
    }

    private getHtmlForWebview(webview: vscode.Webview): string {
        return `
            <!DOCTYPE html>
            <html lang="en">
            <head>
                <meta charset="UTF-8">
                <meta name="viewport" content="width=device-width, initial-scale=1.0">
                <title>Soup Editor</title>
                <style>
                    body {
                        font-family: var(--vscode-font-family);
                        padding: 20px;
                        color: var(--vscode-foreground);
                        background-color: var(--vscode-editor-background);
                    }
                    .kv-pair {
                        display: flex;
                        margin-bottom: 8px;
                        align-items: center;
                    }
                    .kv-key {
                        width: 150px;
                        font-weight: bold;
                    }
                    .kv-value {
                        flex-grow: 1;
                    }
                    input {
                        width: 100%;
                        background: var(--vscode-input-background);
                        color: var(--vscode-input-foreground);
                        border: 1px solid var(--vscode-input-border);
                        padding: 4px;
                    }
                </style>
            </head>
            <body>
                <div id="editor"></div>
                <script>
                    const vscode = acquireVsCodeApi();
                    const editor = document.getElementById('editor');

                    window.addEventListener('message', event => {
                        const message = event.data;
                        switch (message.type) {
                            case 'update':
                                updateEditor(message.text);
                                break;
                        }
                    });

                    function updateEditor(text) {
                        editor.innerHTML = '';
                        const lines = text.split('\\n');
                        let currentLevel = editor;
                        const levels = [editor];

                        lines.forEach((line, index) => {
                            const trimmed = line.trim();
                            if (trimmed === '') return;

                            if (trimmed.endsWith('{')) {
                                const match = line.match(/^(\\s*)([^\\s"]+|"[^"]+")\\s*\\{$/);
                                if (match) {
                                    const [_, indent, key] = match;
                                    const container = document.createElement('div');
                                    container.className = 'container';
                                    container.style.paddingLeft = '16px';
                                    
                                    const title = document.createElement('div');
                                    title.style.fontWeight = 'bold';
                                    title.style.marginTop = '12px';
                                    title.textContent = key + ' {';
                                    
                                    container.appendChild(title);
                                    currentLevel.appendChild(container);
                                    currentLevel = container;
                                    levels.push(container);
                                }
                            } else if (trimmed === '}') {
                                if (levels.length > 1) {
                                    const last = levels.pop();
                                    const closing = document.createElement('div');
                                    closing.textContent = '}';
                                    last.appendChild(closing);
                                    currentLevel = levels[levels.length - 1];
                                }
                            } else {
                                const match = line.match(/^(\\s*)([^\\s"]+|"[^"]+")\\s+(.*)$/);
                                if (match) {
                                    const [_, indent, key, value] = match;
                                    const div = document.createElement('div');
                                    div.className = 'kv-pair';
                                    div.style.paddingLeft = '8px';
                                    
                                    const keySpan = document.createElement('span');
                                    keySpan.className = 'kv-key';
                                    keySpan.textContent = key;
                                    
                                    const input = document.createElement('input');
                                    input.className = 'kv-value';
                                    input.value = value;
                                    input.onchange = () => {
                                        lines[index] = indent + key + ' ' + input.value;
                                        vscode.postMessage({
                                            type: 'change',
                                            text: lines.join('\\n')
                                        });
                                    };
                                    
                                    div.appendChild(keySpan);
                                    div.appendChild(input);
                                    currentLevel.appendChild(div);
                                }
                            }
                        });
                    }
                </script>
            </body>
            </html>
        `;
    }

    private updateTextDocument(document: vscode.TextDocument, text: string) {
        const edit = new vscode.WorkspaceEdit();
        edit.replace(
            document.uri,
            new vscode.Range(0, 0, document.lineCount, 0),
            text
        );
        return vscode.workspace.applyEdit(edit);
    }
}
