# docs/variables'n'functions/[TypeScript]extension.md

## 概要
VS Code拡張機能（TypeScript側）のエントリーポイント。
LSPサーバー（Rust）を起動し、VS Codeエディタとの間でLSP通信を媒介する軽量クライアントとして動作する。
サーバーの起動失敗時にホストがクラッシュするのを防ぐため、エラーハンドリングとログ収集用の出力チャネルを備える。

## 変数定義

### `client` (L11-11) 
- **型**: `LanguageClient | undefined`
- **説明**: 起動したLSPクライアントのインスタンスを保持するグローバル変数。

### `outputChannel` (L12-12)
- **型**: `vscode.OutputChannel`
- **説明**: 拡張機能およびLSPサーバーのデバッグログを出力するためのVS Code出力チャネル。

## 関数定義

### `activate` (L28-129)
- **引数**:
  - `context: vscode.ExtensionContext` - 拡張機能のコンテキストオブジェクト。
- **戻り値**: `void`
- **説明**:
  - 拡張機能がアクティブ化された際にVS Codeより呼び出される。
  - `"Docs Auditor"` という名前の出力チャネルを作成する。
  - RustでビルドされたLSPサーバーバイナリのパスを特定する。
  - セキュリティ向上（Workspace Trust バイパスによるRCEの防止）のため、拡張機能が開発デバッグモード（`vscode.ExtensionMode.Development`）のときのみ、ワークスペースフォルダ内の `server/target/debug/server.exe` または `release/server.exe` を探索してロードします。
  - 通常の配布実行モードでは、ワークスペース内のバイナリは無視し、拡張機能のインストールフォルダ内の `server/target/release/server`（Windowsの場合は `.exe`）を最優先でロードし、存在しない場合のみ `debug` バイナリへフォールバックします。
  - 特定したサーバーバイナリが存在しない場合、エラーを警告表示した上で安全に `activate` 処理を終了（LSPの起動処理を抑止）します。
  - サーバーの起動オプション（ServerOptions）およびクライアントオプション（LanguageClientOptions）を設定する。
  - `LanguageClientOptions` にて以下を設定：
    - `documentSelector` に Markdown、Rust、TypeScript、JavaScript、Python、Go、C、C++、C#、Ruby、Swift、Kotlin、Java を指定し、これらのファイルの変更を監視対象とする。
    - `outputChannel` に作成した出力チャネルを登録。
    - `initializationOptions` に `{ locale: vscode.env.language }` を指定し、エディタの表示言語設定をLSPサーバーに引き渡す。
    - `initializationFailedHandler` を設定し、LSPサーバーの起動や初期化が失敗した際にエラーを出力チャネルへ出力し、拡張機能ホストをクラッシュさせずに安全に終了（`false` を返却）させる。
  - `LanguageClient` インスタンスを生成して起動する。
  - `docsAuditor.autoInjection` 設定変更の監視登録を行う。
  
### `deactivate` (L131-137)
- **引数**: なし
- **戻り値**: `Thenable<void> | undefined`
- **説明**:
  - 拡張機能がクローズまたは無効化される際に呼び出される。
  - `client` が起動していれば、クライアントの停止（`stop`）処理を呼び出す。

### `DocsAuditorSettingsProvider` (クラス)
- **説明**:
  - `vscode.WebviewViewProvider` を実装し、左のアクティビティバーから開くサイドバー用の設定ビューを提供する。
  - **メソッド**:
    - `resolveWebviewView(webviewView: vscode.WebviewView, context: vscode.WebviewViewResolveContext, token: vscode.CancellationToken)`: WebviewView の初期化、HTML の設定、および Webview からの `postMessage` メッセージ受信ハンドラを設定する。
    - `_getHtmlForWebview(webview: vscode.Webview)`: トグルスイッチとステータスを表示するモダンな HTML/CSS を生成する。HTML 内のイベントで `autoInjection` 設定（`docsAuditor.autoInjection`）を変更できるようにする。

## 依存関係マッピング (Dependency Mapping)

```mermaid
graph TD
    activate --> client
    activate --> outputChannel
    activate --> DocsAuditorSettingsProvider
    deactivate --> client
    DocsAuditorSettingsProvider --> resolveWebviewView
    resolveWebviewView --> _getHtmlForWebview
```

## 影響範囲 (Impact Scope)
- 起動時の安全性が向上し、LSPサーバー起動失敗時にも拡張機能開発ホストが巻き込まれてクラッシュするのを防止します。
- 開発時において、ワークスペース内の最新ビルド成果物を自動的に優先ロードするため、拡張機能の再インストールなしで最新の tree-sitter パーサーや監査ロジックを直ちにテスト可能になります。