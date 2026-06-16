import * as vscode from 'vscode';

export interface ExtensionConfig {
    binaryPath: string;
    includeDirs: string[];
    uvmHome: string;
    defines: string[];
}

export function getConfig(): ExtensionConfig {
    const config = vscode.workspace.getConfiguration('hdl-graph');
    return {
        binaryPath: config.get<string>('binaryPath', ''),
        includeDirs: config.get<string[]>('includeDirs', ['rtl', 'tb', 'sim']),
        uvmHome: config.get<string>('uvmHome', ''),
        defines: config.get<string[]>('defines', []),
    };
}

export function onConfigChange(listener: (config: ExtensionConfig) => void): vscode.Disposable {
    return vscode.workspace.onDidChangeConfiguration((e) => {
        if (e.affectsConfiguration('hdl-graph')) {
            listener(getConfig());
        }
    });
}
