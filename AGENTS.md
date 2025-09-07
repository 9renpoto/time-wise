# Repository Guidelines

## プロジェクト構成・配置
- `src/`: Leptos の UI（Rust/WASM）。エントリ `src/main.rs`、主要コンポーネントは `src/app.rs`。
- `src-tauri/`: Tauri バックエンド（Rust）。エントリ `src-tauri/src/main.rs`、トレイ制御等は `src-tauri/src/lib.rs`。
- `index.html` と `Trunk.toml`: Trunk による UI のビルド/配信設定。
- `public/`: 静的アセット。`dist/` と `target/`: ビルド成果物。
- `.github/`: CI ワークフロー（fmt、clippy、test、coverage、release）。

## ビルド・テスト・開発コマンド
- デスクトップ実行: `cargo tauri dev`（Trunk が `:1420` で起動し Tauri を立ち上げ）。
- デスクトップビルド: `cargo tauri build`（`trunk build` 後に Tauri でパッケージ）。
- UI のみ: `trunk serve`（`http://localhost:1420`）、`trunk build`（出力は `dist/`）。
- 整形: `cargo fmt --all -- --check`、静的解析: `cargo clippy --workspace -- -D warnings`。
- テスト: `cargo test --workspace` またはクレート単位（`-p time-wise` / `-p time-wise-ui`）。
- 事前チェック一括: `pre-commit run -a`（cspell, secretlint, cargo-sort, fmt, clippy, test）。

## コーディング規約・命名
- Rust 2021。インデントはスペース4。不要な末尾空白は避ける。
- 命名: 関数/変数/モジュールは snake_case、型/トレイトは PascalCase、定数は SCREAMING_SNAKE_CASE。
- モジュールは小さく焦点化。クレート間共有は `lib.rs` の公開 API を優先。
- 依存は `cargo-sort` で整序、スペルは `cspell` で検査。

## テスト指針
- ユニットテストは同一ファイル内の `#[cfg(test)]`、結合テストは `tests/` を推奨。
- ローカル実行は `cargo test --workspace`。CI で grcov+Codecov によりカバレッジ計測。
- 速く決定的なテストを重視。外部作用はモック化し、FS/ネットワークを単体では避ける。

## コミット・PR ガイド
- Conventional Commits を採用: `feat:`, `fix:`, `chore(deps):`, `refactor:`, `build:`（履歴と整合）。
- PR には要約、関連 Issue、UI 変更はスクリーンショット/GIF を添付。
- CI を必ず通過（fmt, clippy, test）。clippy 警告は不可。

## セキュリティ・設定
- 秘密情報はコミット不可。`secretlint` がローカル/CI で検知。脆弱性報告は `SECURITY.md` を参照。
- Linux では WebKitGTK 等の開発依存が必要な場合あり（CI と同等）。

## エージェント向けメモ
- `src-tauri/gen/` 配下やアイコン類は生成物のため変更しないこと。
- 変更は最小限にし、本ガイドとツール群に整合させること。
