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

    // 開発・デバッグ用：開発デバッグモードの時のみワークスペース内のビルド成果物を優先
    if (context.extensionMode === vscode.ExtensionMode.Development && vscode.workspace.workspaceFolders && vscode.workspace.workspaceFolders.length > 0) {
        const workspaceRoot = vscode.workspace.workspaceFolders[0].uri.fsPath;
        const workspaceDebugPath = path.join(workspaceRoot, 'server', 'target', 'debug', serverExe);
        const workspaceReleasePath = path.join(workspaceRoot, 'server', 'target', 'release', serverExe);

        if (fs.existsSync(workspaceReleasePath)) {
            serverPath = workspaceReleasePath;
        } else if (fs.existsSync(workspaceDebugPath)) {
            serverPath = workspaceDebugPath;
        }
    }

    // 配布用：インストール先フォルダから取得（release優先）
    if (!serverPath) {
        const debugServerPath = context.asAbsolutePath(
            path.join('server', 'target', 'debug', serverExe)
        );
        const releaseServerPath = context.asAbsolutePath(
            path.join('server', 'target', 'release', serverExe)
        );

        if (fs.existsSync(releaseServerPath)) {
            serverPath = releaseServerPath;
        } else if (fs.existsSync(debugServerPath)) {
            serverPath = debugServerPath;
        } else {
            serverPath = releaseServerPath; // デフォルトフォールバック
        }
    }

    logInfo(`[Docs Auditor] LSP サーバーパスを決定しました: ${serverPath}`);

    // サーバーバイナリが存在しない場合は警告を表示して起動を中止
    if (!fs.existsSync(serverPath)) {
        const errorMsg = `Docs Auditor LSP サーバーバイナリが見つかりません。起動を中止します。インストール状態を確認してください: ${serverPath}`;
        logError(errorMsg);
        vscode.window.showWarningMessage(errorMsg);
        return; // 起動を中止
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
        synchronize: {
            configurationSection: 'docsAuditor'
        },
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

    // Webviewビュープロバイダの登録
    const provider = new DocsAuditorSettingsProvider(context.extensionUri);
    context.subscriptions.push(
        vscode.window.registerWebviewViewProvider(
            DocsAuditorSettingsProvider.viewType,
            provider
        )
    );
}

export function deactivate(): Thenable<void> | undefined {
    logInfo('[Docs Auditor] 拡張機能非アクティベート処理を実行します。');
    if (!client) {
        return undefined;
    }
    return client.stop();
}

class DocsAuditorSettingsProvider implements vscode.WebviewViewProvider {
    public static readonly viewType = 'docs-auditor-settings';
    private _view?: vscode.WebviewView;

    constructor(private readonly _extensionUri: vscode.Uri) {}

    public resolveWebviewView(
        webviewView: vscode.WebviewView,
        context: vscode.WebviewViewResolveContext,
        _token: vscode.CancellationToken
    ) {
        this._view = webviewView;

        webviewView.webview.options = {
            enableScripts: true,
            localResourceRoots: [this._extensionUri]
        };

        webviewView.webview.html = this._getHtmlForWebview(webviewView.webview);

        // 設定の変更を監視してWebviewに通知
        const configListener = vscode.workspace.onDidChangeConfiguration((e) => {
            if (e.affectsConfiguration('docsAuditor.autoInjection')) {
                const autoInject = vscode.workspace.getConfiguration('docsAuditor').get<boolean>('autoInjection', false);
                webviewView.webview.postMessage({ type: 'updateState', value: autoInject });
            }
        });

        webviewView.onDidDispose(() => {
            configListener.dispose();
        });

        webviewView.webview.onDidReceiveMessage((data) => {
            switch (data.type) {
                case 'toggleAutoInjection': {
                    vscode.workspace.getConfiguration('docsAuditor').update('autoInjection', data.value, vscode.ConfigurationTarget.Global);
                    break;
                }
            }
        });
    }

    private _getHtmlForWebview(webview: vscode.Webview): string {
        const autoInject = vscode.workspace.getConfiguration('docsAuditor').get<boolean>('autoInjection', false);

        return `<!DOCTYPE html>
<html lang="ja">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <style>
        body {
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
            padding: 16px;
            color: var(--vscode-foreground);
            background-color: var(--vscode-sideBar-background);
            font-size: 13px;
            line-height: 1.6;
        }
        .container {
            display: flex;
            flex-direction: column;
            gap: 16px;
        }
        .header {
            font-weight: 600;
            font-size: 14px;
            border-bottom: 1px solid var(--vscode-divider);
            padding-bottom: 8px;
            color: var(--vscode-settings-headerForeground);
        }
        .setting-item {
            display: flex;
            justify-content: space-between;
            align-items: center;
            background-color: var(--vscode-welcomePage-tileBackground, rgba(255,255,255,0.02));
            padding: 12px;
            border-radius: 8px;
            border: 1px solid var(--vscode-widget-border, rgba(0,0,0,0.1));
            transition: all 0.2s ease;
        }
        .setting-item:hover {
            border-color: var(--vscode-focusBorder);
        }
        .setting-info {
            display: flex;
            flex-direction: column;
            gap: 4px;
            padding-right: 8px;
        }
        .setting-title {
            font-weight: 500;
        }
        .setting-desc {
            font-size: 11px;
            color: var(--vscode-descriptionForeground);
        }
        .switch {
            position: relative;
            display: inline-block;
            width: 40px;
            height: 20px;
            flex-shrink: 0;
        }
        .switch input {
            opacity: 0;
            width: 0;
            height: 0;
        }
        .slider {
            position: absolute;
            cursor: pointer;
            top: 0; left: 0; right: 0; bottom: 0;
            background-color: var(--vscode-button-secondaryBackground, #555);
            transition: .3s cubic-bezier(0.4, 0, 0.2, 1);
            border-radius: 20px;
        }
        .slider:before {
            position: absolute;
            content: "";
            height: 14px;
            width: 14px;
            left: 3px;
            bottom: 3px;
            background-color: white;
            transition: .3s cubic-bezier(0.4, 0, 0.2, 1);
            border-radius: 50%;
            box-shadow: 0 1px 3px rgba(0,0,0,0.3);
        }
        input:checked + .slider {
            background-color: var(--vscode-button-background, #007acc);
        }
        input:checked + .slider:before {
            transform: translateX(20px);
        }
        .status-panel {
            font-size: 11px;
            background-color: var(--vscode-textBlockQuote-background, rgba(0,0,0,0.1));
            border-left: 3px solid var(--vscode-textBlockQuote-border, #007acc);
            padding: 8px 12px;
            border-radius: 0 6px 6px 0;
            color: var(--vscode-descriptionForeground);
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">Docs Auditor 設定</div>
        <div class="setting-item">
            <div class="setting-info">
                <div class="setting-title">自動行数インジェクション</div>
                <div class="setting-desc">仕様書Markdownに行番号を自動で追記します。</div>
            </div>
            <label class="switch">
                <input type="checkbox" id="autoInjectCheck" ${autoInject ? 'checked' : ''}>
                <span class="slider"></span>
            </label>
        </div>
        <div class="status-panel">
            設定を変更すると、ウィンドウの再読み込みなしで即座にコードの整合性スキャンに反映されます。
        </div>
    </div>

    <script>
        const vscode = acquireVsCodeApi();
        const checkbox = document.getElementById('autoInjectCheck');

        checkbox.addEventListener('change', (event) => {
            vscode.postMessage({
                type: 'toggleAutoInjection',
                value: event.target.checked
            });
        });

        window.addEventListener('message', event => {
            const message = event.data;
            if (message.type === 'updateState') {
                checkbox.checked = message.value;
            }
        });
    </script>
</body>
</html>`;
    }
}


