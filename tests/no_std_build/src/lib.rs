//! Verifies that `sensitive-fmt`'s generated code builds in a `no_std + alloc`
//! environment. We can't easily run a no_std binary as a normal test, so this
//! is a build-only check: if `cargo build` succeeds for this sub-crate, the
//! `no_std` claim holds.

#![no_std]

extern crate alloc;

use alloc::string::String;

use sensitive_fmt::{SensitiveDebug, SensitiveDisplay};

pub struct NotFormattable;

#[derive(SensitiveDebug, SensitiveDisplay)]
pub struct Record {
    pub id: u64,
    pub label: &'static str,
    #[sensitive(redact)]
    pub email: String,
    #[sensitive(truncate = 4)]
    pub token: String,
    #[sensitive(skip)]
    pub blob: NotFormattable,
}

/// A tiny helper to verify the generated impls really are usable from no_std
/// code by writing into a `core::fmt::Write` sink.
pub fn render<W: core::fmt::Write>(r: &Record, w: &mut W) -> core::fmt::Result {
    #[allow(unused_imports)]
    use core::fmt::Write as _;
    write!(w, "{:?}", r)?;
    write!(w, " | ")?;
    write!(w, "{}", r)
}
