import * as vscode from 'vscode';
import { LspClient } from './client';
import { BinaryManager } from './binary_manager';
import { GraphPanel } from './graph_panel';

let lspClient: LspClient | undefined;

export async function activate(context: vscode.ExtensionContext) {
    console.log('HDL Code Graph extension activating...');

    // Binary download/management
    const binaryManager = new BinaryManager(context);
    const binaryPath = await binaryManager.ensure();

    // LSP client
    lspClient = new LspClient(context, binaryPath);
    lspClient.start();

    // Register commands
    context.subscriptions.push(
        vscode.commands.registerCommand('hdl-graph.showHierarchy', () => {
            GraphPanel.show(context, 'hierarchy');
        }),
        vscode.commands.registerCommand('hdl-graph.showCallGraph', () => {
            GraphPanel.show(context, 'callgraph');
        }),
        vscode.commands.registerCommand('hdl-graph.showTLMConnections', () => {
            GraphPanel.show(context, 'tlm');
        }),
        vscode.commands.registerCommand('hdl-graph.resetIndex', () => {
            lspClient?.stop();
            lspClient?.start();
            vscode.window.showInformationMessage('HDL Graph index reset');
        })
    );

    console.log('HDL Code Graph extension activated');
}

export function deactivate() {
    lspClient?.stop();
}
