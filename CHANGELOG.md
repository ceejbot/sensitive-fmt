# Changelog

## Unreleased

### Fixed

- Reject `#[sensitive(...)]` on structs, enums, and variants with a compile
  error. rustc accepts derive helper attributes anywhere inside the item, so a
  misplaced attribute used to be a silent no-op that printed the field in full.
- Print raw-identifier field names without the `r#` prefix (`type:` rather
  than `r#type:`), matching std's `#[derive(Debug)]`.
- Recognize associated-type shorthand (`T::Value`) when synthesizing bounds.
  Such fields used to get no predicate and fail with E0277.
- Treat the `Self` keyword as recursive in bound synthesis. A field type
  naming both a parameter and `Self`, such as `(T, Box<Self>)`, used to emit a
  self-referential predicate and fail with E0275.
- Bind enum variant fields to positional locals inside generated match arms,
  so a field named `__f` or `__builder` no longer shadows generated locals.

### Changed

- The Display derive now routes `truncate` through the same local marker
  enum the Debug derive uses, so the generated code holds exactly one copy of
  the truncation algorithm. Output is unchanged.
- Document the attribute grammar on the crate and on both derives, so
  docs.rs shows it without a trip to the README.
- Audit workflow now runs weekly, watches the deny config that actually
  exists, and drops the default token permissions. Test workflow pins the
  nextest and llvm-cov binaries it downloads.

## 0.2.1 — 2026-09-04

### Changed

- Upgrade `syn` from 2 to 3. The only source change is the rename of the
  function-pointer type variant (`Type::BareFn` → `Type::FnPtr`) in bound
  synthesis. Generated code, error messages, features, and the 1.88.0 MSRV
  are unchanged.
- Re-bless three `compile_fail` snapshots for rustc 1.98's reworded
  "conditionally implemented" diagnostics. Test-only; no behavior change.

## 0.2.0 — 2026-06-28

### Added

- Add enum support for unit variants and named-field variants. Tuple variants
  still fail at compile time with a targeted diagnostic.
- Add field-type bound synthesis for generic inputs, including associated types,
  recursive `Self` references, and `PhantomData<T>`-style fields.

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
