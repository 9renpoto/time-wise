# ADR 0007: Use hk for project hooks

- Status: Accepted
- Date: 2026-10-29

Use hk for repository Git hooks. Keep shared linters in the user's global hk
configuration and define only project-specific Cargo checks in `hk.pkl`.
Preserve the existing file-based pre-push selection for Clippy and tests.

The project configuration requires hk 2.7.1 or newer. Contributors must install
hk and run `hk install`. The former `.pre-commit-config.yaml` and prek hook
shims are removed.

## Consequences

- zizmor, Biome, typos, and staged gitleaks checks come from the global hk
  configuration and are not duplicated in this repository.
- Cargo sort and formatting checks run at pre-commit. Clippy and tests continue
  to run at pre-push only for the affected workspace area.
- Contributors need the shared global hk configuration for the shared linters.
  The repository configuration remains focused on the checks that vary by
  project.

<details>
<summary>日本語</summary>

## 決定

リポジトリの Git hook 管理に hk を使う。共通 lint はユーザーの hk
グローバル設定に置き、`hk.pkl` にはプロジェクト固有の Cargo チェックだけを定義する。
Clippy と test は既存どおり、変更ファイルの範囲に応じて pre-push で実行する。

プロジェクト設定が要求する hk の最低バージョンは 2.7.1 とする。開発者は hk を導入し、
`hk install` を実行する。`.pre-commit-config.yaml` と prek の hook shim は削除する。

## 影響

- zizmor、Biome、typos、staged 対象の gitleaks はグローバル hk 設定から実行し、
  このリポジトリでは重複定義しない。
- Cargo sort と format は pre-commit で実行する。Clippy と test は、変更された
  workspace 領域に対応するものだけを pre-push で実行する。
- 共通 lint には共有グローバル hk 設定が必要となる。リポジトリ側の設定は、
  プロジェクトごとに異なるチェックに集中する。

</details>
