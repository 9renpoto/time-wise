# Time Wise

[![CI](https://github.com/9renpoto/time-wise/actions/workflows/ci.yml/badge.svg)](https://github.com/9renpoto/time-wise/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/9renpoto/time-wise/graph/badge.svg?token=Fwk8L6cDp3)](https://codecov.io/gh/9renpoto/time-wise)
[![standard-readme compliant](https://img.shields.io/badge/readme%20style-standard-brightgreen.svg?style=flat-square)](https://github.com/RichardLitt/standard-readme)

A desktop app that records focused-application time and keeps usage history on your device.

Time Wise shows where your time goes without collecting window titles, document
names, browsing URLs, or browser tabs. Usage history remains on the computer and
can be deleted from the app.

Windows packages are currently published through GitHub Releases. The macOS app
is supported by the codebase, but its release pipeline is currently paused.
Linux is not a supported product target.

## Table of Contents

- [Background](#background)
- [Install](#install)
- [Usage](#usage)
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

Download the latest Windows installer from
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

## Documentation

The [user guide](https://9renpoto.github.io/time-wise/) is available in Japanese
and English. Its Markdown sources live in `docs/src/` and are generated with
mdBook:

```bash
mdbook build docs
mdbook build docs/en
```

The generated `docs/book/` directory is not tracked on the source branch.
Changes merged into `main` are built and published to the `gh-pages` branch.

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

Install the repository hooks with `prek install --overwrite` and
`prek install --overwrite --hook-type pre-push`. Include screenshots or a GIF
when changing the desktop UI.

## License

[MIT](LICENSE) © 2026 9renpoto
