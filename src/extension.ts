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
    
    let serverPath = '';

    // ワークスペースフォルダが存在する場合、ワークスペース内のビルド成果物を優先（開発・デバッグ用）
    if (vscode.workspace.workspaceFolders && vscode.workspace.workspaceFolders.length > 0) {
        const workspaceRoot = vscode.workspace.workspaceFolders[0].uri.fsPath;
        const workspaceDebugPath = path.join(workspaceRoot, 'server', 'target', 'debug', serverExe);
        const workspaceReleasePath = path.join(workspaceRoot, 'server', 'target', 'release', serverExe);

        if (fs.existsSync(workspaceDebugPath)) {
            serverPath = workspaceDebugPath;
        } else if (fs.existsSync(workspaceReleasePath)) {
            serverPath = workspaceReleasePath;
        }
    }

    // ワークスペース内で見つからない場合は、インストール先フォルダから取得（配布用）
    if (!serverPath) {
        const debugServerPath = context.asAbsolutePath(
            path.join('server', 'target', 'debug', serverExe)
        );
        const releaseServerPath = context.asAbsolutePath(
            path.join('server', 'target', 'release', serverExe)
        );

        serverPath = debugServerPath;
        if (!fs.existsSync(serverPath) && fs.existsSync(releaseServerPath)) {
            serverPath = releaseServerPath;
        }
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
            { scheme: 'file', language: 'go' },
            { scheme: 'file', language: 'c' },
            { scheme: 'file', language: 'cpp' },
            { scheme: 'file', language: 'csharp' },
            { scheme: 'file', language: 'ruby' },
            { scheme: 'file', language: 'swift' },
            { scheme: 'file', language: 'kotlin' },
            { scheme: 'file', language: 'java' }
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


