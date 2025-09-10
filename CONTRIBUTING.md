# Contributing to Time Wise

Thanks for your interest in contributing! This guide explains the local setup, coding standards, and the pull request process for this repository.

## Getting Started

- Prerequisites
  - Rust (Edition 2021) via `rustup`.
  - Add the WASM target: `rustup target add wasm32-unknown-unknown`.
  - Trunk and Tauri CLI: `cargo install trunk tauri-cli --locked`.
  - Linux only: system deps such as WebKitGTK may be required (match CI).
- Install hooks (recommended): `pipx install pre-commit` then `pre-commit install`.

## Project Structure

- `src/`: Leptos UI (Rust/WASM). Entry `src/main.rs`; main component `src/app.rs`.
- `src-tauri/`: Tauri backend (Rust). Entry `src-tauri/src/main.rs`; tray control in `src-tauri/src/lib.rs`.
- `index.html`, `Trunk.toml`: Trunk build/serve settings.
- `public/`: Static assets. Build outputs in `dist/` and `target/`.

## Development

- Desktop app (UI + Tauri): `cargo tauri dev` (Trunk on `http://localhost:1420`).
- UI only: `trunk serve` (dev) or `trunk build` (outputs to `dist/`).
- Format: `cargo fmt --all -- --check`.
- Lint: `cargo clippy --workspace -- -D warnings`.
- Tests: `cargo test --workspace` (or per crate: `-p time-wise` / `-p time-wise-ui`).
- Pre-flight locally: `pre-commit run -a` (cspell, secretlint, cargo-sort, fmt, clippy, test).

## Coding Standards

- Style: 4-space indent; avoid trailing whitespace.
- Naming: snake_case for functions/variables/modules; PascalCase for types/traits; SCREAMING_SNAKE_CASE for constants.
- Keep modules small and focused. Share across crates via `lib.rs` public APIs.
- Maintain dependency order with `cargo-sort`; check spelling with `cspell`.

## Commit Style

- Use Conventional Commits, for example:
  - `feat: add tray menu actions`
  - `fix: handle empty state in app`
  - `chore(deps): bump leptos to 0.8`
  - `refactor: extract time utils`
  - `build: enable release profile tweaks`

## Pull Requests

- Include a clear summary and link related issues.
- For UI changes, attach screenshots/GIFs.
- Ensure CI will pass by running locally: `pre-commit run -a`.
- No clippy warnings. Keep changes minimal and focused.
- Update docs when behavior or commands change.

## Security

- Do not commit secrets. `secretlint` runs locally and in CI.
- Report vulnerabilities via the process in `SECURITY.md`.

## Generated/Do-Not-Edit Areas

- Do not modify generated artifacts under `src-tauri/gen/` or icon assets.

## Questions

- Check `AGENTS.md` for repository-specific guidelines.
- If something is unclear, open a discussion or a draft PR to get early feedback.

