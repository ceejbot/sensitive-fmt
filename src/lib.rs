//! Derive `Debug` and `Display` for Rust structs while honoring sensitive
//! field attributes. Designed for keeping PHI, PII, secrets, and tokens out
//! of log lines.
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
//! See the [README](https://github.com/ceejbot/sensitive-fmt) for the full
//! attribute reference.

use proc_macro::TokenStream;

mod codegen;
mod parse;

#[proc_macro_derive(SensitiveDebug, attributes(sensitive))]
pub fn derive_sensitive_debug(input: TokenStream) -> TokenStream {
    let ast = syn::parse_macro_input!(input as syn::DeriveInput);
    match parse::plan_struct(&ast, "SensitiveDebug") {
        Ok(fields) => codegen::emit_debug_impl(&ast, &fields).into(),
        Err(err) => err.to_compile_error().into(),
    }
}

#[proc_macro_derive(SensitiveDisplay, attributes(sensitive))]
pub fn derive_sensitive_display(input: TokenStream) -> TokenStream {
    let ast = syn::parse_macro_input!(input as syn::DeriveInput);
    match parse::plan_struct(&ast, "SensitiveDisplay") {
        Ok(fields) => codegen::emit_display_impl(&ast, &fields).into(),
        Err(err) => err.to_compile_error().into(),
    }
}
