# Changelog

## 0.1.1 — 2026-04-30

### Changed

- Improve enum-input compile error: drop the `in 0.x` qualifier from the
  message. The crate's lack of enum support is a design choice, not a
  pre-1.0 limitation.
- Hoist `extern crate alloc` out of per-field blocks in the generated
  `Display` impl. Cosmetic; emitted code is unchanged in behavior.
- Tighten CI: `cargo clippy --all-targets` now runs with `-D warnings`.

### Removed

- Drop the `extra-traits` feature from the `syn` dependency. It was only
  pulled in to support unused `#[derive(Debug)]` annotations on internal
  types; removing it shrinks proc-macro compile time.

## 0.1.0 — 2026-04-29

Initial release. Two derives — `SensitiveDebug`, `SensitiveDisplay` —
honoring `#[sensitive(redact | truncate = N | skip)]`. `no_std`
compatible; `truncate` requires `alloc`. MSRV 1.88.0.
