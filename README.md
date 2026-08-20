# Time Wise

[![CI](https://github.com/9renpoto/time-wise/actions/workflows/ci.yml/badge.svg)](https://github.com/9renpoto/time-wise/actions/workflows/ci.yml)
[![codecov](https://codecov.io/github/umeno3/time-wise/graph/badge.svg?token=AN6oYXhAyl)](https://codecov.io/github/umeno3/time-wise)

Time Wise is a ScreenTime clone built with Tauri and Leptos. Thanks to Tauri's
multi-platform runtime, the desktop app runs on macOS, Windows, and Linux with
a shared code base.

This template should help get you started developing with Tauri and Leptos.

## Recommended IDE Setup

[VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer).

---

## Install

To get started, you need to have [rustup](https://rustup.rs/) and the Tauri CLI installed.

Follow the instructions on the [Tauri website](https://tauri.app/v1/guides/getting-started/prerequisites) to set up your environment.

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

## License

This project is licensed under the terms of the [LICENSE](./LICENSE) file.
