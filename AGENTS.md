# AGENTS.md

Rust CLI (`trslat`) that translates between Chinese and English. Single crate, edition 2024. Backends: Bing (default), Google (`-a`).

## Commands
- Build: `cargo build` / release `cargo build --release`
- Test: `cargo test` (unit tests in `src/i18n.rs`, `src/bing.rs`, and `main.rs`)
- Lint: `cargo clippy` (configured via `[lints.clippy] all = "warn"`)
- After a release build, the installed binary is a **symlink** `~/.local/bin/trslat -> target/release/trslat`. Rebuilding release is enough to update it; no install step.

## i18n gotchas (easy to break)

- Messages live in `locales/en/main.ftl` and `locales/zh-CN/main.ftl`; **a new key must be added to BOTH files**.
- `src/i18n.rs::localize()` maps CLI help text via `arg-*` keys. Field names use `_`, keys use `-` (`from_stdin` -> `arg-from-stdin`). Helpment for CLI args comes from these ftl keys, **not** from doc comments (those are low-value placeholders).
- Add/update messages in both locale files AND the matching `assert_eq!` in the i18n test module — missing one fails tests.
- Every user-facing error string must go through `i18n::t()` / `render_error()`; **never** hardcode language in `bing.rs`/`google.rs`. `TranslateError` (in `src/error.rs`) carries structured data and `i18n::render_error()` localizes it.

## Conventions
- Architecture: `main.rs` (CLI + dispatch), `error.rs` (typed `TranslateError`), `i18n.rs` (locale detect + Fluent), `bing.rs`, `google.rs`.
- No `unsafe`. Keep locale detection logic in a **pure** `pick()` so tests avoid `std::env::set_var` (unsafe in edition 2024).
- stdin is auto-detected (reads when stdin is not a terminal); no `-f` flag.
- **Cleaning dead code: deleting/altering a feature means removing it everywhere** — main.rs struct field, `localize()` arg_keys, BOTH ftl files, and the i18n test. Syncing all back is a recurring bug.
- Commits: English, conventional format (`feat:`/`fix:`/`refactor:`/`chore:`).
- Version bumps: update `version` in `Cargo.toml` **and** `Cargo.lock`, bump according to semver, and tag `v<version>`.

## Release flow (manual from history)
`cargo build --release` -> `ln -sf "$PWD/target/release/trslat" ~/.local/bin/trslat` -> verify. Tags pushed to GitHub via `git push origin <tag>`.