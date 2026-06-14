# Design Spec: README.md and AGENTS.md Creation

## 1. 目的
GitHubでの公開を見据え、本プロジェクトの目的、動作要件、高度なAST解析機能、AIエージェント向けのシステムプロンプト（`AGENTS.md`）をわかりやすく解説する `README.md` と、AI協調開発ルールを明記した `AGENTS.md` を作成する。

## 2. AGENTS.md の設計
AIエージェントがDocs Auditorを使用するプロジェクトで動作する際、正しく仕様書とコードの整合性を保つためのルール（プロンプト）を記載する。
- 項目：仕様書の作成ルール、型定義や引数の一致、依存関係（Mermaid図）の作成、`variables_functions_audit_report.md` がある場合のTODO解消義務。

## 3. README.md の構成設計
日本語と英語で並列に記述し、アンカーリンクで切り替えられるようにする。
- タイトル: `Docs Auditor`
- 一言概要（日・英）
- SVGバッジ（MIT License, Languages, Protocol/Compatibility）
- 日本語セクション / Englishセクション：
  - **概要**：LSPによる自動整合性監査の解説。
  - **動作環境**：
    - VSCodeおよびAntigravity IDEでの動作確認済み。
    - Windows 11環境で検証済み。macOS/Linux環境でも動作可能と想定されますが、現時点で検証は未実施である旨を明記。
    - Rust製LSPサーバーバイナリ同梱。
  - **対応プログラミング言語**：Rust, TS/JS, Python, Go, C/C++, C#, Ruby, Swift, Kotlin, Java。
  - **主な機能**：
    - ASTベースのリアルタイム構造解析（Tree-sitter）
    - 双方向の整合性照合（シンボル有無、型、引数、戻り値、依存関係）
    - 行番号の自動追記・同期（クイックフィックス / 自動インジェクション）
    - 監査レポート（`variables_functions_audit_report.md`）の自動ライフサイクル管理
    - 多言語対応（10言語ローカライズ）
  - **AIエージェントとの協調開発 (AI Co-development)**：`AGENTS.md` の同梱意図と、他のAIツール（Cursor等）で使うためのインテグレーション方法（コピペ案内）。
  - **インストール方法**：VSIXインストール手順、ソースからのビルド手順。
  - **設定項目**：`docsAuditor.autoInjection` 等の設定。
  - **対応ロケールと免責事項**：10言語リスト、AI翻訳に対する免責（Disclaimer）。
  - **ライセンス**：MIT License（Copyright: YuzukoUnderson）。
