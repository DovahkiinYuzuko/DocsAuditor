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

function logInfo(message: string) {
    if (outputChannel) {
        outputChannel.appendLine(message);
    }
    console.log(message);
}

function logError(message: string) {
    if (outputChannel) {
        outputChannel.appendLine(`[ERROR] ${message}`);
    }
    console.error(message);
}

export function activate(context: vscode.ExtensionContext) {
    // 出力チャネルを作成し、即座にログを出力できるようにする
    outputChannel = vscode.window.createOutputChannel('Docs Auditor');
    logInfo('[Docs Auditor] 拡張機能アクティベート処理を開始しました。');

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

    logInfo(`[Docs Auditor] LSP サーバーパスを決定しました: ${serverPath}`);

    // サーバーバイナリが存在しない場合は警告を表示
    if (!fs.existsSync(serverPath)) {
        const errorMsg = `Docs Auditor LSP サーバーバイナリが見つかりません。Rustコードをビルドしてください: ${serverPath}`;
        logError(errorMsg);
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
            { scheme: 'file', language: 'rust' },
            { scheme: 'file', language: 'typescript' },
            { scheme: 'file', language: 'javascript' },
            { scheme: 'file', language: 'python' },
            { scheme: 'file', language: 'go' }
        ],
        outputChannel: outputChannel,
        initializationOptions: {
            locale: vscode.env.language
        },
        initializationFailedHandler: (error) => {
            logError(`LSP サーバー初期化に失敗しました: ${error}`);
            // false を返して再試行しないようにする
            return false;
        }
    };

    logInfo('[Docs Auditor] LanguageClient インスタンスを作成しています...');
    client = new LanguageClient(
        'docsAuditor',
        'Docs Auditor',
        serverOptions,
        clientOptions
    );

    logInfo('[Docs Auditor] LanguageClient を起動しています...');
    client.start().then(() => {
        logInfo('[Docs Auditor] LSP サーバーが正常に起動・接続されました。');
    }).catch((error) => {
        logError(`LSP サーバーの起動中に致命的なエラーが発生しました: ${error}`);
    });
}

export function deactivate(): Thenable<void> | undefined {
    logInfo('[Docs Auditor] 拡張機能非アクティベート処理を実行します。');
    if (!client) {
        return undefined;
    }
    return client.stop();
}


