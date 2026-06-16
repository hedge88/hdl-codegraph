import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';
import * as https from 'https';

const REPO = 'lixiaoxin/hdl-codegraph';
const VERSION = '0.1.0';

export class BinaryManager {
    constructor(private context: vscode.ExtensionContext) {}

    async ensure(): Promise<string> {
        const configPath = vscode.workspace.getConfiguration('hdl-graph').get<string>('binaryPath', '');
        if (configPath && fs.existsSync(configPath)) {
            return configPath;
        }

        const platform = this.getPlatform();
        const binaryDir = path.join(this.context.globalStorageUri.fsPath, 'bin');
        const binaryPath = path.join(binaryDir, this.getBinaryName());

        if (fs.existsSync(binaryPath)) {
            return binaryPath;
        }

        // Auto-download
        fs.mkdirSync(binaryDir, { recursive: true });
        const url = `https://github.com/${REPO}/releases/download/v${VERSION}/hdl-graph-${platform}.tar.gz`;

        vscode.window.withProgress({
            location: vscode.ProgressLocation.Notification,
            title: 'Downloading hdl-graph binary...',
        }, async () => {
            await this.download(url, binaryPath);
            fs.chmodSync(binaryPath, 0o755);
        });

        return binaryPath;
    }

    private getPlatform(): string {
        const arch = process.arch === 'arm64' ? 'aarch64' : 'x86_64';
        switch (process.platform) {
            case 'darwin': return `${arch}-apple-darwin`;
            case 'linux': return `${arch}-unknown-linux-gnu`;
            case 'win32': return 'x86_64-pc-windows-msvc';
            default: return 'x86_64-unknown-linux-gnu';
        }
    }

    private getBinaryName(): string {
        return process.platform === 'win32' ? 'hdl-graph.exe' : 'hdl-graph';
    }

    private download(url: string, dest: string): Promise<void> {
        return new Promise((resolve, reject) => {
            https.get(url, (res) => {
                if (res.statusCode !== 200) {
                    reject(new Error(`Download failed: ${res.statusCode}`));
                    return;
                }
                const file = fs.createWriteStream(dest);
                res.pipe(file);
                file.on('finish', () => { file.close(); resolve(); });
            }).on('error', reject);
        });
    }
}
