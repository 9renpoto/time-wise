# Time Wise

[![CI](https://github.com/umeno3/time-wise/actions/workflows/ci.yml/badge.svg)](https://github.com/umeno3/time-wise/actions/workflows/ci.yml)
[![codecov](https://codecov.io/github/umeno3/time-wise/graph/badge.svg?token=AN6oYXhAyl)](https://codecov.io/github/umeno3/time-wise)

Time Wise is a Screen Time-inspired desktop app built with Tauri v2 and Leptos.
Windows and macOS are the supported and distributed v1 platforms. Linux keeps
the shared portable core and an adapter boundary, but is not officially
supported.

## Project management

Tasks and progress are tracked in [GitHub Issues](https://github.com/9renpoto/time-wise/issues).
Product and technical decisions are recorded in [architecture decision records](./docs/adr/).

## Recommended IDE Setup

[VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer).

---

## Install

To get started, you need to have [rustup](https://rustup.rs/) and the Tauri CLI installed.

Follow the [Tauri v2 prerequisites](https://v2.tauri.app/start/prerequisites/) to set up your environment.

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
