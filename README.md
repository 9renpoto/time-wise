# Time Wise

[![CI](https://github.com/9renpoto/time-wise/actions/workflows/ci.yml/badge.svg)](https://github.com/9renpoto/time-wise/actions/workflows/ci.yml)
[![coverage](https://9renpoto.github.io/time-wise/badges/coverage.svg)](https://github.com/9renpoto/time-wise/actions/workflows/ci.yml)
[![test execution time](https://9renpoto.github.io/time-wise/badges/test-time.svg)](https://github.com/9renpoto/time-wise/actions/workflows/ci.yml)
[![standard-readme compliant](https://img.shields.io/badge/readme%20style-standard-brightgreen.svg?style=flat-square)](https://github.com/RichardLitt/standard-readme)

A desktop app that records focused-application time and keeps usage history on your device.

Time Wise shows where your time goes without collecting window titles, document
names, browsing URLs, or browser tabs. Usage history remains on the computer and
can be deleted from the app.

Windows and macOS packages are built from version tags and published through
GitHub Releases. macOS releases also generate a Homebrew cask for Intel and
Apple Silicon Macs.
Linux is not a supported product target.

## Table of Contents

- [Background](#background)
- [Install](#install)
- [Usage](#usage)
- [Self-hosting](#self-hosting)
- [Documentation](#documentation)
- [Contributing](#contributing)
- [License](#license)

## Background

Time Wise measures how long each desktop application remains in focus. It is
designed for reviewing daily and weekly application usage while limiting the
data collected to what is required for those views.

The desktop application is built with Tauri and Leptos. It records usage while
running in the system tray, excludes time while the screen is locked or the
computer is asleep, and stores history in an on-device SQLite database.

## Install

After the first stable release and tap setup, install on macOS with:

```bash
brew tap 9renpoto/tap
brew install --cask 9renpoto/tap/time-wise
```

To update, run `brew update && brew upgrade --cask time-wise`.

Download the latest Windows installer or macOS DMG from
[GitHub Releases](https://github.com/9renpoto/time-wise/releases). See the
[user guide](https://9renpoto.github.io/time-wise/) for installation and
first-run instructions in Japanese and English.

To build the project locally, first install:

- [Rust through rustup](https://rustup.rs/). The required toolchain and WebAssembly
  target are pinned in `rust-toolchain.toml`.
- The [Tauri system prerequisites](https://v2.tauri.app/start/prerequisites/).
  Linux development additionally requires WebKitGTK and libayatana-appindicator.
- The Tauri CLI and Trunk.

Then clone and build the workspace:

```bash
git clone https://github.com/9renpoto/time-wise.git
cd time-wise
cargo build --workspace
```

## Usage

Start the desktop application with live reload:

```bash
cd apps/desktop
cargo tauri dev
```

Create a distributable desktop build:

```bash
cd apps/desktop
cargo tauri build
```

To work on the web UI without the desktop shell, run `trunk serve` from
`apps/desktop`.

For local Cloudflare runtime development alongside the desktop app, see the
[celld development environment](tools/celld/README.md).

<details>
<summary>日本語</summary>

desktopと並行してCloudflare互換runtimeをローカル実行する手順は、
[celld開発環境](tools/celld/README.md)を参照してください。

</details>

## Self-hosting

Deploy the authentication **foundation** to your own Cloudflare account:

[![Deploy to Cloudflare](https://deploy.workers.cloudflare.com/button)](https://deploy.workers.cloudflare.com/?url=https%3A%2F%2Fgithub.com%2F9renpoto%2Ftime-wise)

Follow the [self-hosting guide](docs/self-hosting.md) for origin/profile settings,
D1 setup and deployment checks. Authentication currently remains disabled; this
button does not yet provide working GitHub login or desktop synchronization.

<details>
<summary>日本語</summary>

認証基盤を自分のCloudflareへ配置できます。origin/profile・D1・検証は
[セルフホスト手順](docs/self-hosting.md)を参照してください。
現時点では認証は無効で、GitHub loginやdesktop同期は利用できません。

</details>

## Documentation

The [user guide](https://9renpoto.github.io/time-wise/) is available in Japanese
and English. Its Markdown sources live in `docs/src/` and are generated with
mdBook:

```bash
mdbook build docs
mdbook build docs/en
```

The generated `docs/book/` directory is not tracked on the source branch.
Documentation and Octocov badges are built from `main` and deployed together to
GitHub Pages through Actions. The coverage and test-time badges use measured CI
results; source files are not updated by the publisher.

<details>
<summary>日本語</summary>

ドキュメントとOctocovのバッジは、`main` からビルドし、Actions経由でGitHub Pagesへまとめて公開します。
カバレッジとテスト実行時間のバッジはCIの計測結果を使い、公開処理によるソースファイルの更新は行いません。

</details>

## Contributing

Questions, bug reports, and feature requests are welcome in
[GitHub Issues](https://github.com/9renpoto/time-wise/issues). Pull requests are
accepted and should use Conventional Commit titles.

Before opening a pull request, run the checks relevant to your change. For
cross-workspace changes, run the full verification set:

```bash
cargo fmt --all -- --check
cargo clippy --workspace -- -D warnings
cargo test --workspace
```

Install hk 2.7.1 or newer, then run `hk install` to enable the repository
hooks. Include screenshots or a GIF when changing the desktop UI.

<details>
<summary>日本語</summary>

hk 2.7.1 以降をインストールし、`hk install` を実行してリポジトリの hook を有効にしてください。
デスクトップ UI を変更する場合は、スクリーンショットまたは GIF を添付してください。

</details>

Release maintainers: see [the tag-based release guide](docs/releasing.md) for
tap setup, version preparation, and publishing.

## License

[MIT](LICENSE) © 2026 9renpoto
