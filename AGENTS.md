# Repository Guidelines / リポジトリガイドライン

## プロジェクト構成とモジュール配置
- `src/`: Leptos UI。エントリは `main.rs`、主要コンポーネントは `app.rs` に実装し、機能ごとに小さく分割します。
- `src-tauri/`: Tauri バックエンド。エントリは `src/main.rs`、トレイ制御や共有コマンドは `src/lib.rs` に配置します。
- `public/` は静的アセット、`dist/` と `target/` はビルド成果物でコミット禁止です。
- CI・リリース設定は `.github/` 配下。`src-tauri/gen/` など生成物は手動編集しないでください。

## ビルド・テスト・開発コマンド
- `cargo tauri dev`: Trunk が `http://localhost:1420` で UI を提供しつつデスクトップアプリを起動します。
- `trunk serve` / `trunk build`: UI のみを開発・ビルドし、成果物は `dist/` に出力されます。
- `cargo tauri build`: `trunk build` 後にデスクトップ配布物を生成します。
- `cargo fmt --all -- --check`、`cargo clippy --workspace -- -D warnings`、`cargo test --workspace`: コミット前必須チェック。`pre-commit run -a` で cspell や secretlint もまとめて実行できます。

## コーディングスタイルと命名
- Rust 2021、インデントはスペース4、不要な末尾空白は除去。既存が ASCII の場合は ASCII を維持します。
- 命名規則: 関数・変数・モジュールは `snake_case`、型・トレイトは `PascalCase`、定数は `SCREAMING_SNAKE_CASE`。
- 依存関係は `cargo-sort` で整序し、`cargo fmt` と `cargo clippy` を常に通してからレビューを依頼します。
- CSS クラス名は BEM（Block__Element--Modifier）方式で命名します。

## テストガイドライン
- 高速で決定的なテストを優先し、ユニットテストは同ファイルの `#[cfg(test)]`、結合テストは `tests/` に配置します。
- `cargo test --workspace` をプッシュ前に必ず実行。CI では grcov+Codecov によるカバレッジが走るため、FS やネットワーク依存は最小化してください。

## コミットとプルリクエスト
- Conventional Commits を採用（例: `feat: add pomodoro timer`, `fix: handle tray errors`）。
- PR には簡潔な概要、関連 Issue、UI 変更がある場合はスクリーンショットや GIF を添付します。
- fmt・clippy・test を通過した状態でレビューを依頼し、大きな設計変更は PR 説明に背景と意図を明記してください。

## セキュリティと構成のヒント
- 機密情報はコミット禁止。`secretlint` が自動検知します。脆弱性報告手順は `SECURITY.md` を参照。
- Linux 環境では WebKitGTK など CI 相当のシステム依存が必要になる場合があるため、事前に整備してください。

## エージェント向けメモ
- 既存のユーザー変更を尊重し、無関係な差分を戻さないでください。
- 生成ファイル（アイコン類や `src-tauri/gen/`）には触れず、変更は最小限かつ必要に応じて簡潔なコメントを添えます。
- 既存の公開 API を優先的に再利用し、ロジックの重複を避けて理解しやすい差分を保ちましょう。
