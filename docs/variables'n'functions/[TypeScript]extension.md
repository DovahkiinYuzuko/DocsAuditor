# docs/variables'n'functions/[TypeScript]extension.md

## 概要
VS Code拡張機能（TypeScript側）のエントリーポイント。
LSPサーバー（Rust）を起動し、VS Codeエディタとの間でLSP通信を媒介する軽量クライアントとして動作する。
サーバーの起動失敗時にホストがクラッシュするのを防ぐため、エラーハンドリングとログ収集用の出力チャネルを備える。

## 変数定義

### `client`
- **型**: `LanguageClient | undefined`
- **説明**: 起動したLSPクライアントのインスタンスを保持するグローバル変数。

### `outputChannel`
- **型**: `vscode.OutputChannel`
- **説明**: 拡張機能およびLSPサーバーのデバッグログを出力するためのVS Code出力チャネル。

## 関数定義

### `activate`
- **引数**:
  - `context: vscode.ExtensionContext` - 拡張機能のコンテキストオブジェクト。
- **戻り値**: `void`
- **説明**:
  - 拡張機能がアクティブ化された際にVS Codeより呼び出される。
  - `"Docs Auditor"` という名前の出力チャネルを作成する。
  - RustでビルドされたLSPサーバーバイナリのパス（通常は `server/target/release/server` またはデバッグバイナリ）を特定する。
  - サーバーの起動オプション（ServerOptions）およびクライアントオプション（LanguageClientOptions）を設定する。
  - `LanguageClientOptions` にて以下を設定：
    - `documentSelector` に Markdown、Rust、TypeScript、JavaScript、Python、Go を指定し、これらのファイルの変更を監視対象とする。
    - `outputChannel` に作成した出力チャネルを登録。
    - `initializationOptions` に `{ locale: vscode.env.language }` を指定し、エディタの表示言語設定をLSPサーバーに引き渡す。
    - `initializationFailedHandler` を設定し、LSPサーバーの起動や初期化が失敗した際にエラーを出力チャネルへ出力し、拡張機能ホストをクラッシュさせずに安全に終了（`false` を返却）させる。
  - `LanguageClient` インスタンスを生成して起動する。
  - `docsAuditor.autoInjection` 設定変更の監視登録を行う。

### `deactivate`
- **引数**: なし
- **戻り値**: `Thenable<void> | undefined`
- **説明**:
  - 拡張機能がクローズまたは無効化される際に呼び出される。
  - `client` が起動していれば、クライアントの停止（`stop`）処理を呼び出す。

## 依存関係マッピング (Dependency Mapping)

```mermaid
graph TD
    activate --> client
    activate --> outputChannel
    deactivate --> client
```

## 影響範囲 (Impact Scope)
- 起動時の安全性が向上し、LSPサーバー起動失敗時にも拡張機能開発ホストが巻き込まれてクラッシュするのを防止します。
