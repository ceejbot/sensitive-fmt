//! Derive `Debug` and `Display` for Rust structs and enums while honoring
//! sensitive field attributes. Designed for keeping PHI, PII, secrets, and
//! tokens out of log lines.
//!
//! # Example
//!
//! ```
//! use sensitive_fmt::{SensitiveDebug, SensitiveDisplay};
//!
//! #[derive(SensitiveDebug, SensitiveDisplay)]
//! struct Patient {
//!     id: u64,
//!     #[sensitive(truncate = 4)] mrn: String,
//!     #[sensitive(redact)]       email: String,
//! }
//!
//! let p = Patient {
//!     id: 42,
//!     mrn: "MRN-12345-WXYZ".into(),
//!     email: "alice@example.com".into(),
//! };
//! assert_eq!(
//!     format!("{p}"),
//!     "Patient { id: 42, mrn: ****WXYZ, email: REDACTED }",
//! );
//! ```
//!
//! # Field attributes
//!
//! One attribute, three mutually exclusive modifiers. The attribute goes on
//! fields only; placing it on a struct, enum, or variant is a compile error.
//!
//! | Attribute                    | Output                                                          | Bound added to the field type      |
//! | ---------------------------- | --------------------------------------------------------------- | ---------------------------------- |
//! | *(none)*                     | normal `Debug` / `Display`                                      | `Debug` or `Display` per derive    |
//! | `#[sensitive(redact)]`       | `REDACTED`                                                      | none                               |
//! | `#[sensitive(truncate = N)]` | `****` + last N code points, or `REDACTED` if shorter than N    | `Display` in both derives          |
//! | `#[sensitive(skip)]`         | `<skipped>`                                                     | none                               |
//!
//! Enums with unit and named-field variants are supported; output is prefixed
//! with the variant name, as std's `#[derive(Debug)]` does. Tuple structs,
//! tuple variants, and unions are rejected at compile time.
//!
//! The crate is `no_std` compatible. Only `truncate` requires `alloc`.
//!
//! See the [README](https://github.com/ceejbot/sensitive-fmt) for the
//! comparison with other crates.

use proc_macro::TokenStream;

mod codegen;
mod parse;

/// Derive `core::fmt::Debug`, honoring `#[sensitive(redact | truncate = N |
/// skip)]` on fields.
///
/// Unmarked fields are formatted with `Debug` and need a `Debug` impl. Marked
/// fields never touch the value except `truncate`, which formats it with
/// `Display` and keeps the last N code points. `{:#?}` pretty-printing works.
///
/// ```
/// use sensitive_fmt::SensitiveDebug;
///
/// #[derive(SensitiveDebug)]
/// struct Session {
///     user: String,
///     #[sensitive(truncate = 4)]
///     token: String,
/// }
///
/// let s = Session { user: "alice".into(), token: "sk_live_abc123wxyz".into() };
/// assert_eq!(format!("{s:?}"), r#"Session { user: "alice", token: ****wxyz }"#);
/// ```
#[proc_macro_derive(SensitiveDebug, attributes(sensitive))]
pub fn derive_sensitive_debug(input: TokenStream) -> TokenStream {
    let ast = syn::parse_macro_input!(input as syn::DeriveInput);
    match parse::plan_input(&ast, "SensitiveDebug") {
        Ok(plan) => codegen::emit_debug_impl(&ast, &plan).into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Derive `core::fmt::Display`, honoring `#[sensitive(redact | truncate = N |
/// skip)]` on fields.
///
/// Output has the same `Name { field: value, .. }` shape as the `Debug`
/// derive, but unmarked fields are formatted with `Display` (so strings have
/// no quotes) and need a `Display` impl. Marked fields render identically in
/// both derives.
///
/// ```
/// use sensitive_fmt::SensitiveDisplay;
///
/// #[derive(SensitiveDisplay)]
/// struct Session {
///     user: String,
///     #[sensitive(redact)]
///     token: String,
/// }
///
/// let s = Session { user: "alice".into(), token: "sk_live_abc123wxyz".into() };
/// assert_eq!(format!("{s}"), "Session { user: alice, token: REDACTED }");
/// ```
#[proc_macro_derive(SensitiveDisplay, attributes(sensitive))]
pub fn derive_sensitive_display(input: TokenStream) -> TokenStream {
    let ast = syn::parse_macro_input!(input as syn::DeriveInput);
    match parse::plan_input(&ast, "SensitiveDisplay") {
        Ok(plan) => codegen::emit_display_impl(&ast, &plan).into(),
        Err(err) => err.to_compile_error().into(),
    }
}
