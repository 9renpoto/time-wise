# Time Wise

[![CI](https://github.com/9renpoto/time-wise/actions/workflows/ci.yml/badge.svg)](https://github.com/9renpoto/time-wise/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/9renpoto/time-wise/graph/badge.svg?token=Fwk8L6cDp3)](https://codecov.io/gh/9renpoto/time-wise)

[Documentation](https://9renpoto.github.io/time-wise/) (日本語 / English)

Time Wise is a ScreenTime clone built with Tauri and Leptos. The v1 desktop app
officially supports Windows and macOS with a shared code base. Linux is not an
officially supported target; the project only preserves an adapter boundary for
future support.

This template should help get you started developing with Tauri and Leptos.

## Recommended IDE Setup

[VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer).

---

## Install

To get started, you need to have [rustup](https://rustup.rs/) and the Tauri CLI installed.

Follow the instructions on the [Tauri website](https://v2.tauri.app/start/prerequisites/) to set up your environment.

The Rust toolchain itself is declared in `rust-toolchain.toml`, so rustup installs the correct channel, components, and the `wasm32-unknown-unknown` target on the first build. You do not need to select a toolchain manually.

Once the prerequisites are installed, you can clone the repository and install the dependencies:

```bash
# You need to replace <repository-url> with the actual URL
git clone <repository-url>
# You need to replace <repository-name> with the actual name
cd <repository-name>
cargo build
```

## Usage

To run the application in development mode:

```bash
cargo tauri dev
```

To build the application for production:

```bash
cargo tauri build
```

## Documentation

The user guide is generated with mdBook in Japanese and English:

```bash
mdbook build docs
mdbook build docs/en
```

Preview either language locally with `mdbook serve docs` or `mdbook serve docs/en`.
Changes under `docs/src/` are verified in pull requests. Updates on `main` are
built into the untracked `docs/book/` directory and published only to the
`gh-pages` branch by `.github/workflows/docs.yml`.

## License

This project is licensed under the terms of the [LICENSE](./LICENSE) file.
