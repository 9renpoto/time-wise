# Tauri + Leptos

[![codecov](https://codecov.io/gh/9renpoto/time-wise/graph/badge.svg?token=AN6oYXhAyl)](https://codecov.io/gh/9renpoto/time-wise)

This template should help get you started developing with Tauri and Leptos.

## Recommended IDE Setup

[VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer).

---

## Install

To get started, you need to have Rust and the Tauri CLI installed.

Follow the instructions on the [Tauri website](https://tauri.app/v1/guides/getting-started/prerequisites) to set up your environment.

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

### Running and Verifying the Application

The application is configured to run as a menu bar / system tray application.

1.  **First-Time Setup:** Before running for the first time, you may need to add the WebAssembly (WASM) target for Rust:
    ```bash
    rustup target add wasm32-unknown-unknown
    ```

2.  **Run the App:** Launch the application from your terminal:
    ```bash
    cargo tauri dev
    ```

3.  **Verification Steps:**
    *   When the application starts, **no main window will appear**.
    *   Look for a **new icon** in your operating system's menu bar (macOS) or system tray (Windows/Linux).
    *   **Left-click** the icon. A small, borderless window should appear, displaying a dummy graph of application usage.
    *   **Right-click** the icon. A context menu with a "Quit" option should appear. Clicking "Quit" will terminate the application.

To build the application for production:

```bash
cargo tauri build
```

## License

This project is licensed under the terms of the [LICENSE](./LICENSE) file.
