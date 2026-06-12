# docs/variables'n'functions/[Rust]locker.md

## 概要
自動インジェクション時のファイル書き込み競合（無限保存ループなど）を防止するため、ファイルベースの簡易的な排他ロック（スピンロック）を提供するモジュール。
アトミックなファイル作成機能を利用し、OS依存のない堅牢なロック管理を提供する。

## データ構造定義

### `FileLocker` (構造体)
ロックの状態を保持するリソース管理（RAIIパターン）オブジェクト。
- **フィールド**:
  - `lock_file_path: std::path::PathBuf` - 作成されたロックファイルのパス。

## 関数・メソッド定義

### `try_lock`
- **引数**:
  - `target_path: &std::path::Path` - ロックしたい対象ファイルのパス。
- **戻り値**: `Option<FileLocker>`
- **説明**:
  - 対象ファイルに対応するロックファイル（例: `target_path.with_extension("lock")`）のアトミックな新規作成を試みる。
  - `std::fs::OpenOptions::new().write(true).create_new(true).open(...)` を使用して、ファイルが既に存在しない場合のみ新規作成する。
  - 作成に成功した場合は `Some(FileLocker)` を返し、既に存在して失敗した場合は `None` を返す。

### `FileLocker` の `Drop` トレイト実装
- **説明**:
  - `FileLocker` インスタンスがスコープを抜ける際（破棄される際）に、対応するロックファイルをファイルシステムから削除（`std::fs::remove_file`）して、安全かつ確実にロックを解放する。

## 依存関係マッピング (Dependency Mapping)

```mermaid
graph TD
    try_lock --> FileLocker
    FileLocker --> Drop
```

## 影響範囲 (Impact Scope)
- 新規追加ファイルのため、既存ファイルへの影響なし。
