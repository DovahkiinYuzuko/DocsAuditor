import * as path from 'path';
import * as fs from 'fs';
import * as vscode from 'vscode';
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
    TransportKind
} from 'vscode-languageclient/node';

let client: LanguageClient | undefined;

export function activate(context: vscode.ExtensionContext) {
    const serverExe = process.platform === 'win32' ? 'server.exe' : 'server';
    
    // 開発時のデバッグバイナリパス
    const debugServerPath = context.asAbsolutePath(
        path.join('server', 'target', 'debug', serverExe)
    );

    // リリースビルドのバイナリパス
    const releaseServerPath = context.asAbsolutePath(
        path.join('server', 'target', 'release', serverExe)
    );

    let serverPath = debugServerPath;
    if (!fs.existsSync(serverPath) && fs.existsSync(releaseServerPath)) {
        serverPath = releaseServerPath;
    }

    // サーバーバイナリが存在しない場合は警告を表示
    if (!fs.existsSync(serverPath)) {
        vscode.window.showWarningMessage(
            `Docs Auditor LSP サーバーバイナリが見つかりません。Rustコードをビルドしてください: ${serverPath}`
        );
    }

    const run: ServerOptions = {
        command: serverPath,
        transport: TransportKind.stdio
    };
    
    const serverOptions: ServerOptions = {
        run,
        debug: run
    };

    const clientOptions: LanguageClientOptions = {
        documentSelector: [
            { scheme: 'file', language: 'markdown' },
            { scheme: 'file', language: 'rust' }
        ]
    };

    client = new LanguageClient(
        'docsAuditor',
        'Docs Auditor',
        serverOptions,
        clientOptions
    );

    client.start();
}

export function deactivate(): Thenable<void> | undefined {
    if (!client) {
        return undefined;
    }
    return client.stop();
}
