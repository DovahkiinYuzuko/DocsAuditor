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
let outputChannel: vscode.OutputChannel | undefined;

export function activate(context: vscode.ExtensionContext) {
    // 出力チャネルを作成し、即座にログを出力できるようにする
    outputChannel = vscode.window.createOutputChannel('Docs Auditor');
    outputChannel.appendLine('[Docs Auditor] 拡張機能アクティベート処理を開始しました。');

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

    outputChannel.appendLine(`[Docs Auditor] LSP サーバーパスを決定しました: ${serverPath}`);

    // サーバーバイナリが存在しない場合は警告を表示
    if (!fs.existsSync(serverPath)) {
        const errorMsg = `Docs Auditor LSP サーバーバイナリが見つかりません。Rustコードをビルドしてください: ${serverPath}`;
        outputChannel.appendLine(`[Docs Auditor] ERROR: ${errorMsg}`);
        vscode.window.showWarningMessage(errorMsg);
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
        ],
        outputChannel: outputChannel,
        initializationFailedHandler: (error) => {
            outputChannel?.appendLine(`[Docs Auditor] LSP サーバー初期化に失敗しました: ${error}`);
            // false を返して再試行しないようにする
            return false;
        }
    };

    outputChannel.appendLine('[Docs Auditor] LanguageClient インスタンスを作成しています...');
    client = new LanguageClient(
        'docsAuditor',
        'Docs Auditor',
        serverOptions,
        clientOptions
    );

    outputChannel.appendLine('[Docs Auditor] LanguageClient を起動しています...');
    client.start().then(() => {
        outputChannel?.appendLine('[Docs Auditor] LSP サーバーが正常に起動・接続されました。');
    }).catch((error) => {
        outputChannel?.appendLine(`[Docs Auditor] LSP サーバーの起動中に致命的なエラーが発生しました: ${error}`);
    });
}

export function deactivate(): Thenable<void> | undefined {
    if (outputChannel) {
        outputChannel.appendLine('[Docs Auditor] 拡張機能非アクティベート処理を実行します。');
    }
    if (!client) {
        return undefined;
    }
    return client.stop();
}

