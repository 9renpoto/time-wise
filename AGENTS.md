# Repository Guidelines

## Project Structure & Module Organization
- `src/`: Leptos client UI. `main.rs` chooses the dashboard or settings window (`/?view=settings`). Components live in `presentation/`, domain types in `domain/`, and shared services in `application/` and `infrastructure/`; keep these layers aligned with Clean Architecture principles.
- `src-tauri/`: Desktop backend. `src/lib.rs` manages the tray, startup metrics persistence, and settings window spawning. `src/startup_metrics.rs` stores launch timings in SQLite; `src/main.rs` wires the builder.
- `public/`: Static assets served by Trunk. Build artifacts in `dist/` and `target/` remain untracked. CI, release, and hooks live under `.github/`.

## Build, Test, and Development Commands
- `cargo tauri dev` – Launch the desktop shell with live-reloaded UI at `http://localhost:1420`.
- `trunk serve` / `trunk build` – Develop or bundle the Web UI without the shell.
- `cargo check` • `cargo fmt --all` • `cargo clippy --workspace -- -D warnings` – Fast validation, formatting, and lint gates; run before every commit.
- `cargo test --workspace` – Execute all unit tests, including presentation helpers and backend utilities.
- `cargo tauri build` – Produce distributable binaries (runs `trunk build` first).
- `cargo doc --workspace --no-deps` – Refresh Rustdoc; public comments must be English.

## Coding Style & Naming Conventions
- Rust 2021, four-space indentation, ASCII unless legacy files require otherwise. Use `snake_case` for functions/modules, `PascalCase` for types/traits, and `SCREAMING_SNAKE_CASE` for constants.
- CSS follows BEM (`app__startup-title`). Avoid inline styles except for dynamic values calculated in Leptos.
- Sort manifests with `cargo sort -w`, and rely on existing services in `application/` or `infrastructure/` before adding new helpers.

## Testing Guidelines
- Co-locate unit tests in the same file under `#[cfg(test)]`; integration tests belong in `tests/`.
- Keep tests deterministic and lightweight—mock IO when possible.
- Run `cargo test --workspace` before every PR; CI computes coverage via grcov/Codecov and will flag gaps.

## Commit & Pull Request Guidelines
- Follow Conventional Commits (`feat:`, `fix:`, `chore:`). Example: `feat: add settings tray entry`.
- PRs need a succinct summary, related issues, and screenshots or GIFs for UI updates such as the settings window.
- Verify `fmt`, `clippy`, and `test` locally. Capture non-obvious decisions—like window positioning or database schema changes—in the PR body to streamline review.

## Security & Configuration Tips
- Never commit secrets; `lefthook run pre-commit` runs Biome checks and Rust formatters on staged files.
- Linux contributors must install WebKitGTK and libappindicator (see `README.md`) before running `cargo tauri dev` to match CI requirements.
