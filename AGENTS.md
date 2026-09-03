# Repository Guidelines

## Project Structure & Module Organization
- `apps/desktop/`: Conventional Tauri application root. The Leptos client lives in `src/`, while the native shell, tray, and persistence live in `src-tauri/`. Static assets are in `public/`.
- `apps/server/`: Reserved server package. Keep framework and transport choices out until the server implementation is designed.
- Build artifacts in `apps/desktop/dist/` and `target/` remain untracked. CI, release, and hooks live under `.github/`.

## Build, Test, and Development Commands
- `rust-toolchain.toml` pins the channel, components, and the `wasm32-unknown-unknown` target. Rustup resolves it automatically, so do not rely on a local `rustup default`; CI and releases read the same file.
- `cd apps/desktop && cargo tauri dev` – Launch the conventional `src-tauri` desktop shell with live-reloaded UI at `http://localhost:1420`.
- `cd apps/desktop && trunk serve` / `trunk build` – Develop or bundle the Web UI without the shell.
- `cargo check` • `cargo fmt --all` • `cargo clippy --workspace -- -D warnings` – Full-workspace validation commands; use them after shared manifest or cross-package changes.
- `cargo test --workspace` – Execute all unit tests, including presentation helpers and backend utilities.
- `cd apps/desktop && cargo tauri build` – Produce distributable binaries (runs the web build first).
- `cargo doc --workspace --no-deps` – Refresh Rustdoc; public comments must be English.

## Coding Style & Naming Conventions
- Rust 2021, four-space indentation, ASCII unless legacy files require otherwise. Use `snake_case` for functions/modules, `PascalCase` for types/traits, and `SCREAMING_SNAKE_CASE` for constants.
- CSS follows BEM (`app__startup-title`). Avoid inline styles except for dynamic values calculated in Leptos.
- Sort manifests with `cargo sort -w`, and rely on existing services in `application/` or `infrastructure/` before adding new helpers.

## Testing Guidelines
- Co-locate unit tests in the same file under `#[cfg(test)]`; integration tests belong in `tests/`.
- Keep tests deterministic and lightweight—mock IO when possible.
- Let prek and CI select package-level tests for isolated changes. Run `cargo test --workspace` for shared manifests, cross-package changes, and release validation; pushes to `main` refresh full-workspace coverage via grcov/Codecov.

## Commit & Pull Request Guidelines
- Follow Conventional Commits (`feat:`, `fix:`, `chore:`). Example: `feat: add settings tray entry`.
- PRs need a succinct summary, related issues, and screenshots or GIFs for UI updates such as the settings window.
- Verify `fmt`, `clippy`, and `test` locally. Capture non-obvious decisions—like window positioning or database schema changes—in the PR body to streamline review.

## Security & Configuration Tips
- Never commit secrets; install prek with `cargo install --locked prek`, then install the hooks with `prek install --overwrite` and `prek install --overwrite --hook-type pre-push`. Prek runs fail-first checks and selects Rust verification from the files changed before pushes.
- Linux contributors must install WebKitGTK and libappindicator (see `README.md`) before running `cargo tauri dev` from `apps/desktop` to match CI requirements.
