# Docs Auditor

An LSP-based tool to automatically audit and sync markdown specifications with code, compatible with VSIX-compliant IDEs. / VSIX対応の各種IDEで動作する、LSPベースの仕様書とコードの自動整合性検証・同期ツール。

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow?style=flat-square&logo=opensourceinitiative&logoColor=white)](LICENSE)
![Rust](https://img.shields.io/badge/Rust-orange?style=flat-square&logo=rust&logoColor=white)
![TypeScript](https://img.shields.io/badge/TypeScript-blue?style=flat-square&logo=typescript&logoColor=white)
![VS Code](https://img.shields.io/badge/VS%20Code-007acc?style=flat-square&logo=visual-studio-code&logoColor=white)


[日本語](#日本語) | [English](#english)

---

## 日本語

### 概要
Docs Auditor は、AIエージェントや開発者が作成する「仕様書（Markdown）」と「実際のソースコード」間の整合性を、AST（抽象構文木）を用いて自動かつ厳密に監査するLSPベースのエディタ拡張機能です。
ドキュメントの陳腐化を防ぎ、常に設計書とソースコードが完全に同期された状態を保ちます。

### 開発目的（なぜ仕様書を作成させるのか）
AIエージェントによる開発において、セッションが切り替わったり、別のAIモデルにタスクが引き継がれたりした場合でも、プロジェクト内の変数や関数の定義・位置を「マップのピン」のように視覚化して、コードベースの構造を素早く把握・理解できるようにすることが目的です。
Docs Auditor は、AIエージェントが作成した仕様書とコードの乖離を自動でチェックし、記述の正確性を担保する役割を果たします。

### 動作環境・前提条件
- **対応IDE**: VSCode および Antigravity IDE（VSIX拡張機能をサポートするIDE）で動作確認済みです。
- **対応OS**: Windows 11 環境で動作検証済みです。macOS および Linux 環境でも動作する見込み（LSPはクロスプラットフォーム仕様）ですが、現時点では実機での検証は未実施です。
- **バイナリ同梱**: GitHub Actionsによるワークフローにより、リリース時に各OS用のサーバーバイナリ（Windows用の `server.exe` または Unix用の `server`）が自動ビルドされ、拡張機能パッケージに同梱されて自動起動します。

### サポートプログラミング言語
以下の 11 言語のソースコードをパースし、仕様書との整合性を監査できます：
- Rust, TypeScript, JavaScript, Python, Go, C, C++, C#, Ruby, Swift, Kotlin, Java

### 監査の仕組みと仕様書の記述条件
本ツールが正常に仕様書をパースして監査するためには、仕様書が以下のフォーマットで記述されている必要があります。

1. **シンボル定義の見出し**：
     検証対象の変数や関数は、Markdown内で `### `シンボル名`` または `### `シンボル名` (Lxx-xx)` 形式の3段見出しで定義してください。
     - 例：`### `login` (L10-20)`
2. **引数と戻り値（関数の場合）**：
     見出しの下に、以下のように記述して型情報や引数の個数を指定します。
     - `- **引数**:`
       - `user: String`
     - `- **戻り値**:`
       - `Result`
  3. **変数の型**：
     変数の場合は、`var_type` として型情報を記述します。
  4. **依存関係マッピング**：
     見出しの下に、Mermaid図を用いて以下のように依存関係を明記します。コード上でこれらのシンボルが実際に使われているかをASTから監査します。
     ```mermaid
     graph TD
         caller_function --> callee_function
     ```

  ### 主な機能
  - **ASTベースのリアルタイム構造解析**：`tree-sitter` を用いてコードをパースし、変数/関数の構造を厳密にチェックします。
  - **双方向整合性監査**：仕様書とコードの間でシンボルの存在、変数/関数の種別、引数の数と型、戻り値の型をチェックします。
  - **行番号の自動同期（インジェクション）**：一致が確認されたシンボルに対し、仕様書の見出しにコードの正確な行番号を追記・更新します。エディタのクイックフィックス（💡）から手動で適用できるほか、設定で完全自動化も可能です（書き込み無限ループを防止するファイルロック機構を内蔵）。
  - **デッドコード（未使用仕様）検出**：仕様書に書かれているが、プロジェクト全体で一度も参照されていないシンボルを検知して警告します。
  - **監査レポート（TODO）の自動生成と削除**：整合性エラーを検出すると、プロジェクトルートに `variables_functions_audit_report.md` を自動生成します。すべてのエラーが解決されると、レポートファイルは自動で削除（クリーンアップ）されます。

  ### AIエージェントとの協調開発（AI連携）
  本ツールは、AIエージェント（Claude Code、Cursor、Antigravity CLIなど）との共同開発プロセスを支援します。
  同梱されている `AGENTS.md` は、AIエージェントに読み込ませるためのシステムプロンプト（開発ルール）です。
  AIと協調開発を行う際は、`AGENTS.md` の中身をプロジェクトのルール設定ファイル（`CLAUDE.md` や `.cursorrules` など）にコピーして使用してください。Docs Auditor の監査結果に基づいてAIが自律的に整合性を維持するように動作します。

  ### インストール方法
  #### VSIXからインストールする（通常）
  1. リリースから `docs-auditor-0.1.0.vsix` ファイルをダウンロードします。
  2. VSCode または Antigravity IDE を開き、「拡張機能」タブから「...（メニュー）」->「VSIXからのインストール...」を選択してファイルを選択します。

  #### ソースコードからビルドする（開発者向け）
  1. 本リポジトリをクローンします。
  2. Rustサーバーをビルドします：
     ```powershell
     cd server; cargo build --release
     ```
  3. 拡張機能をコンパイルし、パッケージングします：
     ```powershell
     npm install; npm run compile
     npx @vscode/vsce package
     ```
     これにより、プロジェクトルートに最新の `docs-auditor-0.1.0.vsix` が生成されます。

  ### 設定項目
  - **`docsAuditor.autoInjection` (boolean)**:
    - `true` (ON)：コードとの一致が確認できた際、仕様書ドキュメントへ自動で行番号を追記（インジェクション）します。
    - `false` (OFF / デフォルト)：エディタ上に「クイックフィックス」として提示し、手動選択時にのみ追記します。

  ### 対応ロケールと免責事項
  本ツールは以下の10言語に対応しています：
  - 英語、日本語、ドイツ語、スペイン語、エストニア語、フランス語、韓国語、ベトナム語、中国語（簡体字）、中国語（繁体字）
  - *免責事項*: ローカライズされた翻訳テキスト（警告文、レポートなど）はAI翻訳を用いて生成されているため、一部不自然な表現が含まれている場合があります。

  ### ライセンス
  本プロジェクトは MIT ライセンスの下で公開されています。詳細は `LICENSE` ファイルを参照してください。
  Copyright (c) 2026 YuzukoUnderson

  ---

  ## English

  ### Introduction
  Docs Auditor is an LSP-based editor extension that automatically and rigorously audits the consistency between markdown specification files and the source code using Abstract Syntax Trees (AST). It prevents specification rot, ensuring that design docs and code are always fully synchronized.

  ### Development Goal (Why generate specifications?)
  In development driven by AI agents, this system aims to visualize variables and functions as "map pins" indicating their definition and locations, enabling agents to quickly understand the codebase even when sessions change or tasks are handed over to different AI models. Docs Auditor automatically checks for discrepancies between the code and these specifications, ensuring description accuracy.

  ### Operating Environment & Prerequisites
  - **Compatible IDEs**: Verified to work on VSCode and Antigravity IDE.
  - **Compatible OS**: Verified on Windows 11. Although anticipated to work on macOS and Linux (due to LSP cross-platform capability), OS-specific verification has not been conducted yet.
  - **Binary Bundling**: A GitHub Actions workflow automatically builds the server binary (`server.exe` for Windows or `server` for Unix) on release, bundling it into the extension package.

  ### Supported Programming Languages
  The following 11 languages are supported:
  - Rust, TypeScript, JavaScript, Python, Go, C, C++, C#, Ruby, Swift, Kotlin, Java

  ### Parsing Logic and Specification Formatting
  For the auditor to parse specifications correctly, they must follow this format:

  1. **Symbol Definition Headings**:
     Variables or functions to audit must be defined in Markdown using level-3 headings: `### `symbol_name`` or `### `symbol_name` (Lxx-xx)`.
     - E.g., `### `login` (L10-20)`
  2. **Parameters & Return Types (for Functions)**:
     Under the heading, specify parameter names and return types:
     - `- **Parameters**:`
       - `user: String`
     - `- **Return Value**:`
       - `Result`
  3. **Variable Type**:
     Specify variable types using `var_type`.
  4. **Dependency Mapping**:
     Use Mermaid diagrams under the heading to map out dependencies. The tool verifies if these symbols are actually used in the code via AST:
     ```mermaid
     graph TD
         caller_function --> callee_function
     ```

  ### Key Features
  - **AST-Based Real-Time Analysis**: Parsed via `tree-sitter` for reliable structural verification beyond simple string matching.
  - **Bidirectional Consistency Checks**: Audits missing symbols, variable/function kinds, parameter count/types, return types, and dependencies.
  - **Line Number Auto-Injection & Sync**: Appends or updates the exact code line numbers in the specification headings. Supports manual triggers via quick-fixes (💡) or fully automated synchronization via configurations (with a built-in file lock to prevent infinite scanning loops).
  - **Dead Code (Unused Specifications) Detection**: Identifies and warns about symbols described in the specs but never referenced in the project.
  - **Automated Report Generation & Cleanup**: Generates a `variables_functions_audit_report.md` TODO report in the project root if inconsistencies are found. Cleans up (deletes) the report automatically once all issues are fixed.

  ### AI Co-development
  This extension is designed to support the collaborative development process with AI coding agents (such as Claude Code, Cursor, or Antigravity CLI).
  The included `AGENTS.md` contains the system prompt rules.
  When co-developing with an AI agent, copy the contents of `AGENTS.md` into your agent configuration files (like `CLAUDE.md` or `.cursorrules`). This will prompt the AI to autonomously keep specification documents updated and correct bugs based on Docs Auditor feedback.

  ### Installation
  #### Installing from VSIX (Standard)
  1. Download the `docs-auditor-0.1.0.vsix` file from the releases page.
  2. Open VSCode or Antigravity IDE, navigate to the Extensions tab, click the "..." menu, and select "Install from VSIX...".

  #### Building from Source (Developers)
  1. Clone this repository.
  2. Build the Rust server:
     ```powershell
     cd server; cargo build --release
     ```
  3. Compile and package the extension:
     ```powershell
     npm install; npm run compile
     npx @vscode/vsce package
     ```
     This generates `docs-auditor-0.1.0.vsix` in the project root.

  ### Configuration
  - **`docsAuditor.autoInjection` (boolean)**:
    - `true`: Automatically injects line numbers into specification documents on match.
    - `false` (default): Presents a quick-fix option in the editor to apply manually.

  ### Supported Locales & Disclaimer
  Supported locales:
  - English, Japanese, German, Spanish, Estonian, French, Korean, Vietnamese, Simplified Chinese, Traditional Chinese
  - *Disclaimer*: Localized messages and reports are generated using AI translation and may contain phrasing inaccuracies.

  ### License
  This project is licensed under the MIT License. See the `LICENSE` file for details.
  Copyright (c) 2026 YuzukoUnderson
