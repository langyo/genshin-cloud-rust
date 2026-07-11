# Changelog

All notable changes to the 空荧酒馆·原神地图 Rust backend (Genshin Map Cloud Rust)
will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Dependencies (dev branch)

- Upgrade the workspace to edition 2024 across all four packages
  (`_utils`, `_database`, `_functions`, `_router`); `rust-toolchain.toml`
  pins stable with rustfmt + clippy.
- Bump cross-major dependencies to their latest stable lines: `reqwest`
  ^0.12 → ^0.13, `redis` ^0.32 → ^1, `axum-extra` ^0.10 → ^0.12,
  `tower-http` ^0.6 → ^0.7, `bcrypt` ^0.17 → ^0.19, `jsonwebtoken` ^9 → ^10,
  `md5` ^0.7 → ^0.8, `oneshot` ^0.1 → ^0.2, `flume` ^0.11 → ^0.12.
- **Strip all `aws-*` crates from the dependency graph.** `reqwest` 0.13
  defaults to the `aws-lc-rs` rustls crypto provider, which forces a native
  `aws-lc-sys` C build that fails on MSVC. The workspace now pins `rustls`
  with `default-features = false` and only the `ring` provider; `reqwest`
  uses `rustls-no-provider` so the single `rustls` workspace dep is the only
  source of the crypto provider across reqwest + sqlx/sea-orm. Verified: no
  `aws-` package remains in `cargo tree`.
- **sea-orm** kept on the ^1 stable line. sea-orm 2.0-rc is available but
  introduces a breaking `UpdateOne`/`ValidatedUpdateOne` API that requires
  porting the `SafeEntityTrait` macro
  (`packages/utils/src/db_operations.rs`) and ~33 business call sites in
  `packages/functions`. That migration is tracked as dev-branch follow-up
  work; `strum` stays on ^0.26 (which sea-orm 1.x requires) until then.
- **minio** kept on ^0.3. minio 0.4 renames `Client` → `MinioClient`/
  `MinioClientBuilder` and changes the bucket-provisioning builder API;
  migrating is tracked as follow-up work.

### Known technical debt (dev branch)

- `cargo clippy --workspace -- -D warnings` surfaces ~6 pre-existing lints
  (a deprecated enum variant, a dead `is_available` method, interior-
  mutability constants in `jwt.rs`, a redundant `Ok?`). These predate the
  dependency upgrade; fixing them is tracked separately. CI runs clippy
  without `-D warnings` until they are resolved.

### Tooling

- Install the `celestia-devtools` commit-msg hook enforcing the org gitmoji
  convention (English subject, capitalized, trailing period).
- Replace the merge commit on `master` with a single squashed commit to keep
  the history linear and compliant with the hook's master-merge-guard.
- Add a `justfile` (verb-first dispatch) that imports the vendored
  `celestia-devtools.just` recipes.
- Add `rust-toolchain.toml` (stable + rustfmt + clippy), `rustfmt.toml`, and
  `.editorconfig` for consistent formatting across contributors.
- Add `.cargo/config.toml` with `git-fetch-with-cli` and the Windows 8 MiB
  stack bump; machine-specific `[patch]` overrides stay in user-level config.
- Add `.gitattributes` to normalize line endings to LF.
- Modernize CI: replace the deprecated `actions-rs` workflow with
  `dtolnay/rust-toolchain`-based `rust.yml`, add a multi-OS `test.yml` with a
  secrets scan, a `docs.yml` for multilingual docs, and `dependabot.yml`.
- Add GitHub community files: `PULL_REQUEST_TEMPLATE.md`, `SECURITY.md`,
  `CODE_OF_CONDUCT.md`, and issue templates.
- Add `deny.toml` (cargo-deny policy) for license and advisory gating.

### Documentation

- Rewrite `ReadMe.md` → `README.md` in the celestia-island multilingual format
  (centered header, badge row, language switcher, quick start, architecture,
  documentation index).
- Lay the groundwork for multilingual docs under `docs/` (English and
  Simplified Chinese first; remaining languages scaffolded).

### Notes

- The commit messages on `master` prior to the hook are a mix of Chinese and
  gitmoji; from the hook-install commit forward, all new commits follow the
  org gitmoji convention (English subject line).
- The `noa` co-author hook is reserved and not installed yet — it requires a
  built `noa` binary and the entelecheia chat-log/aporia configuration, neither
  of which is present in this repo's environment.
