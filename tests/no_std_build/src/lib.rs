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

/// Enum input with unit and named-field variants but no `truncate`. This
/// stays purely `core` — the `extern crate alloc` import only kicks in when a
/// `truncate` field exists. If we accidentally start pulling `alloc` in for
/// the redact/skip path, this sub-crate stops building.
#[derive(SensitiveDebug, SensitiveDisplay)]
pub enum Event {
    Heartbeat,
    Login {
        user_id: u64,
        #[sensitive(redact)]
        token: &'static str,
    },
    Failure {
        code: u32,
        #[sensitive(skip)]
        _detail: NotFormattable,
    },
}

pub fn render_event<W: core::fmt::Write>(e: &Event, w: &mut W) -> core::fmt::Result {
    write!(w, "{:?}", e)?;
    write!(w, " | ")?;
    write!(w, "{}", e)
}
