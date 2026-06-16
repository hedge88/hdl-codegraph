import * as vscode from 'vscode';
import { LanguageClient, ServerOptions, LanguageClientOptions } from 'vscode-languageclient/node';

export class LspClient {
    private client: LanguageClient | undefined;

    constructor(
        private context: vscode.ExtensionContext,
        private binaryPath: string
    ) {}

    start() {
        const config = vscode.workspace.getConfiguration('hdl-graph');
        const includeDirs = config.get<string[]>('includeDirs', []);
        const uvmHome = config.get<string>('uvmHome', '');
        const defines = config.get<string[]>('defines', []);

        const args = ['watch'];
        for (const dir of includeDirs) {
            args.push('--include-dirs', dir);
        }
        if (uvmHome) {
            args.push('--uvm-home', uvmHome);
        }
        for (const def of defines) {
            args.push('--defines', def);
        }

        const serverOptions: ServerOptions = {
            command: this.binaryPath,
            args: args,
        };

        const clientOptions: LanguageClientOptions = {
            documentSelector: [
                { scheme: 'file', language: 'systemverilog' },
                { scheme: 'file', language: 'verilog' },
            ],
            synchronize: {
                fileEvents: vscode.workspace.createFileSystemWatcher('**/*.{sv,svh,v,vh}'),
            },
        };

        this.client = new LanguageClient(
            'hdl-graph-lsp',
            'HDL Code Graph LSP',
            serverOptions,
            clientOptions
        );

        this.client.start();
    }

    stop() {
        this.client?.stop();
    }
}
