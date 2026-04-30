//! Code generation for the `SensitiveDebug` and `SensitiveDisplay` derives.

use std::collections::{BTreeMap, BTreeSet};

use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Generics, Type, WhereClause};

use crate::parse::{FieldPlan, PlannedField};

/// Which derive we're generating bounds for. Determines how Plain fields
/// contribute (Debug for SensitiveDebug, Display for SensitiveDisplay).
#[derive(Clone, Copy)]
enum Derive {
    DebugDerive,
    DisplayDerive,
}

/// Which trait bound a single field needs on the type-params it references.
#[derive(Clone, Copy, PartialEq, Eq)]
enum NeededBound {
    None,
    Debug,
    Display,
    Both,
}

fn merge_bound(a: NeededBound, b: NeededBound) -> NeededBound {
    use NeededBound::*;
    match (a, b) {
        (None, x) | (x, None) => x,
        (Both, _) | (_, Both) => Both,
        (Debug, Display) | (Display, Debug) => Both,
        (Debug, Debug) => Debug,
        (Display, Display) => Display,
    }
}

fn bound_for_field(plan: &FieldPlan, derive: Derive) -> NeededBound {
    use Derive::*;
    use NeededBound::*;
    match (plan, derive) {
        (FieldPlan::Plain, DebugDerive) => Debug,
        (FieldPlan::Plain, DisplayDerive) => Display,
        (FieldPlan::Redact, _) => None,
        (FieldPlan::Skip, _) => None,
        // Truncate always uses Display in both derives (formats with Display
        // then slices the resulting string).
        (FieldPlan::Truncate(_), _) => Display,
    }
}

/// Walk a `syn::Type` recursively and collect every type-parameter ident
/// referenced in it. We do not use `syn::visit` (gated behind a separate
/// Cargo feature) and instead recurse over the `Type` variants we care about.
fn collect_type_params(ty: &Type, type_params: &BTreeSet<String>, found: &mut BTreeSet<String>) {
    match ty {
        // A bare path like `T` or `Vec<T>`.
        Type::Path(tp) => {
            // qself is `<X as Trait>::` prefix — skip qualified paths because
            // they name associated types, not top-level type params.
            if tp.qself.is_none()
                && let Some(seg) = tp.path.segments.first()
            {
                let name = seg.ident.to_string();
                if type_params.contains(&name) {
                    found.insert(name);
                }
            }
            // Recurse into generic arguments (e.g. the `T` in `Vec<T>`).
            for seg in &tp.path.segments {
                if let syn::PathArguments::AngleBracketed(ref ab) = seg.arguments {
                    for arg in &ab.args {
                        if let syn::GenericArgument::Type(inner) = arg {
                            collect_type_params(inner, type_params, found);
                        }
                    }
                }
            }
        }
        // &T and &mut T.
        Type::Reference(r) => {
            collect_type_params(&r.elem, type_params, found);
        }
        // [T; N] and [T].
        Type::Array(a) => collect_type_params(&a.elem, type_params, found),
        Type::Slice(s) => collect_type_params(&s.elem, type_params, found),
        // (A, B, C).
        Type::Tuple(t) => {
            for elem in &t.elems {
                collect_type_params(elem, type_params, found);
            }
        }
        // *const T / *mut T.
        Type::Ptr(p) => collect_type_params(&p.elem, type_params, found),
        // (T) — parenthesized type, rare but valid.
        Type::Paren(p) => collect_type_params(&p.elem, type_params, found),
        // Compiler-injected token-group wrapper, common in proc-macro pipelines.
        Type::Group(g) => collect_type_params(&g.elem, type_params, found),
        // fn(T) -> T — function pointer in callback-holding structs.
        Type::BareFn(f) => {
            for arg in &f.inputs {
                collect_type_params(&arg.ty, type_params, found);
            }
            if let syn::ReturnType::Type(_, ret_ty) = &f.output {
                collect_type_params(ret_ty, type_params, found);
            }
        }
        // Everything else (Infer, Never, ImplTrait, TraitObject, …) — no
        // type params to collect.
        _ => {}
    }
}

fn referenced_type_params(ty: &Type, type_params: &BTreeSet<String>) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    collect_type_params(ty, type_params, &mut found);
    found
}

/// Build the `where` clause for an impl, adding the minimum bounds needed.
fn synthesize_where(generics: &Generics, field_types: &[(&Type, &FieldPlan)], derive: Derive) -> WhereClause {
    let type_params: BTreeSet<String> = generics.type_params().map(|tp| tp.ident.to_string()).collect();

    // For each type-param, merge the needed bounds across every field that
    // references it.
    let mut needed: BTreeMap<String, NeededBound> = BTreeMap::new();
    for (ty, plan) in field_types {
        let referenced = referenced_type_params(ty, &type_params);
        let bound = bound_for_field(plan, derive);
        for name in referenced {
            let entry = needed.entry(name).or_insert(NeededBound::None);
            *entry = merge_bound(*entry, bound);
        }
    }

    // Build (or extend) the where clause.
    let mut wc = generics.where_clause.clone().unwrap_or_else(|| WhereClause {
        where_token: syn::token::Where::default(),
        predicates: syn::punctuated::Punctuated::new(),
    });

    for (name, bound) in needed {
        if matches!(bound, NeededBound::None) {
            continue;
        }
        let ident = syn::Ident::new(&name, proc_macro2::Span::call_site());
        let bounds_ts = match bound {
            NeededBound::Debug => quote! { ::core::fmt::Debug },
            NeededBound::Display => quote! { ::core::fmt::Display },
            NeededBound::Both => quote! { ::core::fmt::Debug + ::core::fmt::Display },
            NeededBound::None => unreachable!(),
        };
        let predicate: syn::WherePredicate = syn::parse_quote! { #ident: #bounds_ts };
        wc.predicates.push(predicate);
    }

    wc
}

/// Emit `impl Debug for #name` honoring each field's `FieldPlan`.
///
/// For empty structs we special-case the body to print `Name {}` (with braces),
/// matching the spec. The non-empty path uses `f.debug_struct(...)` so pretty-
/// printing via `{:#?}` works for free. Marked fields use a local marker enum
/// whose Debug impl writes the bare marker string (e.g. `REDACTED`) without
/// the quotes that `&"REDACTED"` would produce.
pub fn emit_debug_impl(input: &DeriveInput, fields: &[PlannedField]) -> TokenStream {
    let name = &input.ident;
    // We discard the inherited where_clause and synthesize one tailored to
    // this derive: minimum bounds on type params, based on per-field needs.
    let (impl_generics, ty_generics, _inherited_wc) = input.generics.split_for_impl();
    let field_types: Vec<(&syn::Type, &FieldPlan)> = fields.iter().map(|f| (&f.ty, &f.plan)).collect();
    let synthesized_wc = synthesize_where(&input.generics, &field_types, Derive::DebugDerive);
    let name_str = name.to_string();

    let body = if fields.is_empty() {
        let empty_lit = format!("{name_str} {{}}");
        quote! { __f.write_str(#empty_lit) }
    } else {
        let any_marked = fields.iter().any(|f| !matches!(f.plan, FieldPlan::Plain));
        let any_truncate = fields.iter().any(|f| matches!(f.plan, FieldPlan::Truncate(_)));

        let alloc_import = if any_truncate {
            quote! { extern crate alloc as __sensitive_fmt_alloc; }
        } else {
            quote! {}
        };
        let truncated_variant = if any_truncate {
            quote! { Truncated(__sensitive_fmt_alloc::string::String, u32), }
        } else {
            quote! {}
        };
        let truncated_arm = if any_truncate {
            quote! {
                __SensitiveFmtMarkers::Truncated(__s, __n) => {
                    let __count = __s.chars().count();
                    if __count >= *__n as usize {
                        let __nm1 = (*__n as usize) - 1;
                        let __start = __s.char_indices().nth_back(__nm1)
                            .map(|(__i, _)| __i).unwrap_or(0);
                        __mf.write_str("****")?;
                        __mf.write_str(&__s[__start..])
                    } else {
                        __mf.write_str("REDACTED")
                    }
                }
            }
        } else {
            quote! {}
        };
        let marker_decl = if any_marked {
            quote! {
                #alloc_import
                // Local marker enum with a Debug impl that writes bare strings or
                // truncated tails. Scoped to this fmt body so it doesn't leak.
                enum __SensitiveFmtMarkers {
                    Redacted,
                    Skipped,
                    #truncated_variant
                }
                impl ::core::fmt::Debug for __SensitiveFmtMarkers {
                    fn fmt(&self, __mf: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                        match self {
                            __SensitiveFmtMarkers::Redacted => __mf.write_str("REDACTED"),
                            __SensitiveFmtMarkers::Skipped  => __mf.write_str("<skipped>"),
                            #truncated_arm
                        }
                    }
                }
            }
        } else {
            quote! {}
        };
        let field_calls = fields.iter().map(|f| {
            let ident = &f.ident;
            let label = ident.to_string();
            match &f.plan {
                FieldPlan::Plain => quote! {
                    __builder.field(#label, &self.#ident);
                },
                FieldPlan::Redact => quote! {
                    __builder.field(#label, &__SensitiveFmtMarkers::Redacted);
                },
                FieldPlan::Skip => quote! {
                    __builder.field(#label, &__SensitiveFmtMarkers::Skipped);
                },
                FieldPlan::Truncate(n) => {
                    let n_lit = *n;
                    quote! {
                        __builder.field(
                            #label,
                            &__SensitiveFmtMarkers::Truncated(
                                __sensitive_fmt_alloc::format!("{}", &self.#ident),
                                #n_lit,
                            ),
                        );
                    }
                }
            }
        });
        quote! {
            #marker_decl
            let mut __builder = __f.debug_struct(#name_str);
            #(#field_calls)*
            __builder.finish()
        }
    };

    quote! {
        #[automatically_derived]
        impl #impl_generics ::core::fmt::Debug for #name #ty_generics #synthesized_wc {
            fn fmt(&self, __f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                #body
            }
        }
    }
}

/// Emit `impl Display for #name` honoring each field's `FieldPlan`.
///
/// For empty structs we special-case the body to print `Name {}` (with
/// braces), matching the Debug derive and the spec.
pub fn emit_display_impl(input: &DeriveInput, fields: &[PlannedField]) -> TokenStream {
    let name = &input.ident;
    // We discard the inherited where_clause and synthesize one tailored to
    // this derive: minimum bounds on type params, based on per-field needs.
    let (impl_generics, ty_generics, _inherited_wc) = input.generics.split_for_impl();
    let field_types: Vec<(&syn::Type, &FieldPlan)> = fields.iter().map(|f| (&f.ty, &f.plan)).collect();
    let synthesized_wc = synthesize_where(&input.generics, &field_types, Derive::DisplayDerive);
    let name_str = name.to_string();

    let body = if fields.is_empty() {
        // Empty struct: print `Name {}` (with braces), matching the Debug derive
        // and the spec.
        let empty_lit = format!("{name_str} {{}}");
        quote! { __f.write_str(#empty_lit) }
    } else {
        let mut writes: Vec<TokenStream> = Vec::new();
        // Open brace.
        let prefix = format!("{name_str} {{ ");
        writes.push(quote! { __f.write_str(#prefix)?; });
        for (i, f) in fields.iter().enumerate() {
            if i > 0 {
                writes.push(quote! { __f.write_str(", ")?; });
            }
            let label = format!("{}: ", f.ident);
            let ident = &f.ident;
            writes.push(quote! { __f.write_str(#label)?; });
            match &f.plan {
                FieldPlan::Plain => writes.push(quote! {
                    ::core::fmt::Display::fmt(&self.#ident, __f)?;
                }),
                FieldPlan::Redact => writes.push(quote! {
                    __f.write_str("REDACTED")?;
                }),
                FieldPlan::Skip => writes.push(quote! {
                    __f.write_str("<skipped>")?;
                }),
                FieldPlan::Truncate(n) => {
                    let n_lit = *n;
                    writes.push(quote! {
                        {
                            extern crate alloc as __sensitive_fmt_alloc;
                            let __s = __sensitive_fmt_alloc::format!("{}", &self.#ident);
                            let __count = __s.chars().count() as u64;
                            if __count >= #n_lit as u64 {
                                let __nm1 = (#n_lit as usize).saturating_sub(1);
                                let __start = __s.char_indices().nth_back(__nm1)
                                    .map(|(__i, _)| __i).unwrap_or(0);
                                __f.write_str("****")?;
                                __f.write_str(&__s[__start..])?;
                            } else {
                                __f.write_str("REDACTED")?;
                            }
                        }
                    });
                }
            }
        }
        writes.push(quote! { __f.write_str(" }") });
        quote! { #(#writes)* }
    };

    quote! {
        #[automatically_derived]
        impl #impl_generics ::core::fmt::Display for #name #ty_generics #synthesized_wc {
            fn fmt(&self, __f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                #body
            }
        }
    }
}
