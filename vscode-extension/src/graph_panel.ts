import * as vscode from 'vscode';

export class GraphPanel {
    static show(context: vscode.ExtensionContext, viewType: string) {
        const panel = vscode.window.createWebviewPanel(
            'hdlGraph',
            `HDL Graph: ${viewType}`,
            vscode.ViewColumn.Beside,
            { enableScripts: true }
        );

        panel.webview.html = this.getHtml(viewType);
    }

    private static getHtml(viewType: string): string {
        return `<!DOCTYPE html>
<html>
<head>
    <style>
        body { font-family: sans-serif; padding: 16px; }
        h1 { color: var(--vscode-editor-foreground); }
        .placeholder { color: var(--vscode-descriptionForeground); text-align: center; margin-top: 48px; }
    </style>
</head>
<body>
    <h1>HDL Graph: ${viewType}</h1>
    <div class="placeholder">
        <p>Graph visualization coming soon.</p>
        <p>Use the CLI for now: <code>hdl-graph query hierarchy &lt;module&gt;</code></p>
    </div>
</body>
</html>`;
    }
}
