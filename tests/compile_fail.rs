//! Snapshot tests for compile-time error messages.
//!
//! To regenerate `.stderr` snapshots after intentionally changing an error
//! message, run:
//!
//! ```text
//! TRYBUILD=overwrite cargo test --test compile_fail
//! ```
//!
//! Snapshots are pinned to stable rustc's diagnostic style. CI sets
//! `SKIP_TRYBUILD=1` on non-stable matrix entries to skip this test there;
//! see `proc_macro_span` (rust-lang/rust#54725) for why diagnostics drift
//! between toolchains.

#[test]
fn compile_fail_cases() {
    if std::env::var("SKIP_TRYBUILD").as_deref() == Ok("1") {
        return;
    }
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/*.rs");
}
