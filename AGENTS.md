# Repository Guidelines

## Project Structure & Layout

- `src/`: Leptos UI (Rust/WASM). Entry at `src/main.rs`; main component in `src/app.rs`.
- `src-tauri/`: Tauri backend (Rust). Entry at `src-tauri/src/main.rs`; tray control etc. in `src-tauri/src/lib.rs`.
- `index.html` and `Trunk.toml`: Trunk settings for building/serving the UI.
- `public/`: Static assets. `dist/` and `target/`: Build artifacts.
- `.github/`: CI workflows (fmt, clippy, test, coverage, release).

## Build, Test, and Dev Commands

- Desktop run: `cargo tauri dev` (Trunk starts on `:1420` and boots Tauri).
- Desktop build: `cargo tauri build` (runs `trunk build` then packages via Tauri).
- UI only: `trunk serve` (`http://localhost:1420`), `trunk build` (outputs to `dist/`).
- Formatting: `cargo fmt --all -- --check`; static analysis: `cargo clippy --workspace -- -D warnings`.
- Tests: `cargo test --workspace` or per crate (`-p time-wise` / `-p time-wise-ui`).
- Pre-flight checks: `pre-commit run -a` (cspell, secretlint, cargo-sort, fmt, clippy, test).

## Coding Conventions & Naming

- Rust 2021. Indent with 4 spaces. Avoid trailing whitespace.
- Naming: functions/variables/modules use snake_case; types/traits use PascalCase; constants use SCREAMING_SNAKE_CASE.
- Keep modules small and focused. For cross-crate sharing, prefer the public API of `lib.rs`.
- Sort dependencies with `cargo-sort` and check spelling with `cspell`.

## Testing Guidelines

- Place unit tests in the same file under `#[cfg(test)]`; prefer `tests/` for integration tests.
- Run locally with `cargo test --workspace`. CI measures coverage with grcov + Codecov.
- Favor fast, deterministic tests. Mock external effects; avoid filesystem/network in unit tests.

## Commit & PR Guide

- Use Conventional Commits: `feat:`, `fix:`, `chore(deps):`, `refactor:`, `build:` (keep history consistent).
- PRs should include a summary, related issues, and screenshots/GIFs for UI changes.
- CI must pass (fmt, clippy, test). No clippy warnings allowed.

## Security & Environment

- Never commit secrets. `secretlint` runs locally/CI to detect them. For vulnerability reporting, see `SECURITY.md`.
- On Linux, you may need development dependencies such as WebKitGTK (same as CI).

## Notes for Agents

- Do not modify generated artifacts under `src-tauri/gen/` or icon assets.
- Keep changes minimal and aligned with this guide and the toolchain.
