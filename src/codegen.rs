//! Code generation for the `SensitiveDebug` and `SensitiveDisplay` derives.

use std::collections::BTreeSet;

use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{DeriveInput, Generics, Type, WhereClause};

use crate::parse::{FieldPlan, Plan, PlannedField, PlannedVariant, VariantShape};

/// Which derive we're generating bounds for. Determines how Plain fields
/// contribute (Debug for SensitiveDebug, Display for SensitiveDisplay).
#[derive(Clone, Copy)]
enum Derive {
    DebugDerive,
    DisplayDerive,
}

/// Which trait bound a single field type needs.
#[derive(Clone, Copy)]
enum NeededBound {
    None,
    Debug,
    Display,
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

fn collect_type_params(ty: &Type, type_params: &BTreeSet<String>, self_type: &Ident, found: &mut BTreeSet<String>) {
    match ty {
        Type::Path(tp) => {
            if let Some(qself) = &tp.qself {
                collect_type_params(&qself.ty, type_params, self_type, found);
            }

            if tp.qself.is_none()
                && tp.path.segments.len() == 1
                && let Some(seg) = tp.path.segments.first()
            {
                let name = seg.ident.to_string();
                if type_params.contains(&name) {
                    found.insert(name);
                }
            }

            for seg in &tp.path.segments {
                if seg.ident == *self_type {
                    continue;
                }
                collect_type_params_from_path_arguments(&seg.arguments, type_params, self_type, found);
            }
        }
        Type::Reference(r) => collect_type_params(&r.elem, type_params, self_type, found),
        Type::Array(a) => collect_type_params(&a.elem, type_params, self_type, found),
        Type::Slice(s) => collect_type_params(&s.elem, type_params, self_type, found),
        Type::Tuple(t) => {
            for elem in &t.elems {
                collect_type_params(elem, type_params, self_type, found);
            }
        }
        Type::Ptr(p) => collect_type_params(&p.elem, type_params, self_type, found),
        Type::Paren(p) => collect_type_params(&p.elem, type_params, self_type, found),
        Type::Group(g) => collect_type_params(&g.elem, type_params, self_type, found),
        Type::BareFn(f) => {
            for arg in &f.inputs {
                collect_type_params(&arg.ty, type_params, self_type, found);
            }
            if let syn::ReturnType::Type(_, ret_ty) = &f.output {
                collect_type_params(ret_ty, type_params, self_type, found);
            }
        }
        _ => {}
    }
}

fn collect_type_params_from_path_arguments(
    arguments: &syn::PathArguments,
    type_params: &BTreeSet<String>,
    self_type: &Ident,
    found: &mut BTreeSet<String>,
) {
    if let syn::PathArguments::AngleBracketed(ab) = arguments {
        for arg in &ab.args {
            match arg {
                syn::GenericArgument::Type(inner) => {
                    collect_type_params(inner, type_params, self_type, found);
                }
                syn::GenericArgument::AssocType(assoc) => {
                    collect_type_params(&assoc.ty, type_params, self_type, found);
                }
                _ => {}
            }
        }
    }
}

fn referenced_type_params(ty: &Type, type_params: &BTreeSet<String>, self_type: &Ident) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    collect_type_params(ty, type_params, self_type, &mut found);
    found
}

fn contains_self_type(ty: &Type, self_type: &Ident) -> bool {
    match ty {
        Type::Path(tp) => {
            tp.qself
                .as_ref()
                .is_some_and(|qself| contains_self_type(&qself.ty, self_type))
                || tp
                    .path
                    .segments
                    .iter()
                    .any(|seg| seg.ident == *self_type || path_arguments_contain_self(&seg.arguments, self_type))
        }
        Type::Reference(r) => contains_self_type(&r.elem, self_type),
        Type::Array(a) => contains_self_type(&a.elem, self_type),
        Type::Slice(s) => contains_self_type(&s.elem, self_type),
        Type::Tuple(t) => t.elems.iter().any(|elem| contains_self_type(elem, self_type)),
        Type::Ptr(p) => contains_self_type(&p.elem, self_type),
        Type::Paren(p) => contains_self_type(&p.elem, self_type),
        Type::Group(g) => contains_self_type(&g.elem, self_type),
        Type::BareFn(f) => {
            f.inputs.iter().any(|arg| contains_self_type(&arg.ty, self_type))
                || matches!(&f.output, syn::ReturnType::Type(_, ret_ty) if contains_self_type(ret_ty, self_type))
        }
        _ => false,
    }
}

fn path_arguments_contain_self(arguments: &syn::PathArguments, self_type: &Ident) -> bool {
    if let syn::PathArguments::AngleBracketed(ab) = arguments {
        ab.args.iter().any(|arg| match arg {
            syn::GenericArgument::Type(inner) => contains_self_type(inner, self_type),
            syn::GenericArgument::AssocType(assoc) => contains_self_type(&assoc.ty, self_type),
            _ => false,
        })
    } else {
        false
    }
}

/// Build the `where` clause for an impl, adding the minimum bounds needed.
fn synthesize_where(
    generics: &Generics,
    self_type: &Ident,
    field_types: &[(&Type, &FieldPlan)],
    derive: Derive,
) -> WhereClause {
    let type_params: BTreeSet<String> = generics.type_params().map(|tp| tp.ident.to_string()).collect();
    let mut wc = generics.where_clause.clone().unwrap_or_else(|| WhereClause {
        where_token: syn::token::Where::default(),
        predicates: syn::punctuated::Punctuated::new(),
    });

    for (ty, plan) in field_types {
        let bound = bound_for_field(plan, derive);
        if matches!(bound, NeededBound::None) {
            continue;
        }
        let bounds_ts = match bound {
            NeededBound::Debug => quote! { ::core::fmt::Debug },
            NeededBound::Display => quote! { ::core::fmt::Display },
            NeededBound::None => unreachable!(),
        };

        let referenced = referenced_type_params(ty, &type_params, self_type);
        if referenced.is_empty() {
            continue;
        }

        if contains_self_type(ty, self_type) {
            for name in referenced {
                let ident = syn::Ident::new(&name, proc_macro2::Span::call_site());
                let predicate: syn::WherePredicate = syn::parse_quote! { #ident: #bounds_ts };
                wc.predicates.push(predicate);
            }
        } else {
            let predicate: syn::WherePredicate = syn::parse_quote! { #ty: #bounds_ts };
            wc.predicates.push(predicate);
        }
    }

    wc
}

/// Flatten the plan into a single list of (type, plan) pairs across all fields,
/// regardless of whether they live in a struct or in enum variants. Bound
/// synthesis treats the input uniformly: duplicate predicates are harmless.
fn collect_field_types(plan: &Plan) -> Vec<(&Type, &FieldPlan)> {
    match plan {
        Plan::Struct(fields) => fields.iter().map(|f| (&f.ty, &f.plan)).collect(),
        Plan::Enum(variants) => variants
            .iter()
            .flat_map(|v| match &v.shape {
                VariantShape::Unit => &[][..],
                VariantShape::Struct(fields) => &fields[..],
            })
            .map(|f| (&f.ty, &f.plan))
            .collect(),
    }
}

/// Build the `__SensitiveFmtMarkers` declaration block. Returns an empty
/// TokenStream when no field uses redact/skip/truncate.
fn build_marker_decl(any_marked: bool, any_truncate: bool) -> TokenStream {
    if !any_marked {
        return quote! {};
    }
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
}

/// One `__builder.field(...)` call for a single field, parameterized by the
/// access expression (`&self.foo` for structs, `foo` for enum match arms).
fn debug_field_call(field: &PlannedField, access: &TokenStream) -> TokenStream {
    let label = field.ident.to_string();
    match &field.plan {
        FieldPlan::Plain => quote! {
            __builder.field(#label, #access);
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
                        __sensitive_fmt_alloc::format!("{}", #access),
                        #n_lit,
                    ),
                );
            }
        }
    }
}

/// One field's worth of Display writes (label + value), parameterized by the
/// access expression. Caller emits separators and the surrounding braces.
fn display_field_writes(field: &PlannedField, access: &TokenStream) -> TokenStream {
    let label = format!("{}: ", field.ident);
    let value = match &field.plan {
        FieldPlan::Plain => quote! {
            ::core::fmt::Display::fmt(#access, __f)?;
        },
        FieldPlan::Redact => quote! {
            __f.write_str("REDACTED")?;
        },
        FieldPlan::Skip => quote! {
            __f.write_str("<skipped>")?;
        },
        FieldPlan::Truncate(n) => {
            let n_lit = *n;
            quote! {
                {
                    let __s = __sensitive_fmt_alloc::format!("{}", #access);
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
            }
        }
    };
    quote! {
        __f.write_str(#label)?;
        #value
    }
}

/// Body for one named-field group: either "Name {}" (empty) or a
/// `f.debug_struct(name).field(...).finish()` chain. `name_str` is the name
/// shown in output ("Patient" for a struct, "Cat" for an enum variant).
fn emit_debug_named_body(
    name_str: &str,
    fields: &[PlannedField],
    field_access: impl Fn(&Ident) -> TokenStream,
) -> TokenStream {
    if fields.is_empty() {
        let empty_lit = format!("{name_str} {{}}");
        return quote! { __f.write_str(#empty_lit) };
    }
    let calls = fields.iter().map(|f| {
        let access = field_access(&f.ident);
        debug_field_call(f, &access)
    });
    quote! {
        let mut __builder = __f.debug_struct(#name_str);
        #(#calls)*
        __builder.finish()
    }
}

/// Body for one named-field group, Display flavor. Same shape rules as
/// `emit_debug_named_body`, just using `write_str` directly.
fn emit_display_named_body(
    name_str: &str,
    fields: &[PlannedField],
    field_access: impl Fn(&Ident) -> TokenStream,
) -> TokenStream {
    if fields.is_empty() {
        let empty_lit = format!("{name_str} {{}}");
        return quote! { __f.write_str(#empty_lit) };
    }
    let prefix = format!("{name_str} {{ ");
    let mut writes: Vec<TokenStream> = vec![quote! { __f.write_str(#prefix)?; }];
    for (i, f) in fields.iter().enumerate() {
        if i > 0 {
            writes.push(quote! { __f.write_str(", ")?; });
        }
        let access = field_access(&f.ident);
        writes.push(display_field_writes(f, &access));
    }
    writes.push(quote! { __f.write_str(" }") });
    quote! { #(#writes)* }
}

/// Single arm for an enum variant in a Debug match.
fn emit_debug_variant_arm(variant: &PlannedVariant) -> TokenStream {
    let v_ident = &variant.ident;
    let v_str = v_ident.to_string();
    match &variant.shape {
        VariantShape::Unit => quote! {
            Self::#v_ident => __f.write_str(#v_str),
        },
        VariantShape::Struct(fields) => {
            if fields.is_empty() {
                let empty_lit = format!("{v_str} {{}}");
                return quote! {
                    Self::#v_ident {} => __f.write_str(#empty_lit),
                };
            }
            let pat_idents = fields.iter().map(|f| &f.ident);
            let body = emit_debug_named_body(&v_str, fields, |ident| quote! { #ident });
            quote! {
                Self::#v_ident { #(#pat_idents),* } => { #body },
            }
        }
    }
}

/// Single arm for an enum variant in a Display match.
fn emit_display_variant_arm(variant: &PlannedVariant) -> TokenStream {
    let v_ident = &variant.ident;
    let v_str = v_ident.to_string();
    match &variant.shape {
        VariantShape::Unit => quote! {
            Self::#v_ident => __f.write_str(#v_str),
        },
        VariantShape::Struct(fields) => {
            if fields.is_empty() {
                let empty_lit = format!("{v_str} {{}}");
                return quote! {
                    Self::#v_ident {} => __f.write_str(#empty_lit),
                };
            }
            let pat_idents = fields.iter().map(|f| &f.ident);
            let body = emit_display_named_body(&v_str, fields, |ident| quote! { #ident });
            quote! {
                Self::#v_ident { #(#pat_idents),* } => { #body },
            }
        }
    }
}

/// Emit `impl Debug for #name` honoring each field's `FieldPlan`.
///
/// For empty named-field groups (struct or empty struct variant) we
/// special-case the body to print `Name {}` (with braces), matching the spec.
/// Otherwise the body uses `f.debug_struct(...)` so pretty-printing via
/// `{:#?}` works for free. Marked fields use a local marker enum whose Debug
/// impl writes the bare marker string (e.g. `REDACTED`) without the quotes
/// that `&"REDACTED"` would produce. Enums dispatch through a `match self`
/// over variants; the marker enum is declared once at the top of `fmt` and
/// shared across all arms.
pub fn emit_debug_impl(input: &DeriveInput, plan: &Plan) -> TokenStream {
    let name = &input.ident;
    let (impl_generics, ty_generics, _inherited_wc) = input.generics.split_for_impl();
    let field_types = collect_field_types(plan);
    let synthesized_wc = synthesize_where(&input.generics, name, &field_types, Derive::DebugDerive);
    let name_str = name.to_string();

    let any_marked = field_types.iter().any(|(_, p)| !matches!(p, FieldPlan::Plain));
    let any_truncate = field_types.iter().any(|(_, p)| matches!(p, FieldPlan::Truncate(_)));
    let marker_decl = build_marker_decl(any_marked, any_truncate);

    let body = match plan {
        Plan::Struct(fields) => emit_debug_named_body(&name_str, fields, |ident| quote! { &self.#ident }),
        Plan::Enum(variants) => {
            if variants.is_empty() {
                quote! { match *self {} }
            } else {
                let arms = variants.iter().map(emit_debug_variant_arm);
                quote! { match self { #(#arms)* } }
            }
        }
    };

    quote! {
        #[automatically_derived]
        impl #impl_generics ::core::fmt::Debug for #name #ty_generics #synthesized_wc {
            fn fmt(&self, __f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                #marker_decl
                #body
            }
        }
    }
}

/// Emit `impl Display for #name` honoring each field's `FieldPlan`.
///
/// For empty named-field groups (struct or empty struct variant) we
/// special-case the body to print `Name {}` (with braces), matching the Debug
/// derive and the spec. Enums dispatch through a `match self` over variants.
pub fn emit_display_impl(input: &DeriveInput, plan: &Plan) -> TokenStream {
    let name = &input.ident;
    let (impl_generics, ty_generics, _inherited_wc) = input.generics.split_for_impl();
    let field_types = collect_field_types(plan);
    let synthesized_wc = synthesize_where(&input.generics, name, &field_types, Derive::DisplayDerive);
    let name_str = name.to_string();

    let any_truncate = field_types.iter().any(|(_, p)| matches!(p, FieldPlan::Truncate(_)));
    let alloc_import = if any_truncate {
        quote! { extern crate alloc as __sensitive_fmt_alloc; }
    } else {
        quote! {}
    };

    let body = match plan {
        Plan::Struct(fields) => emit_display_named_body(&name_str, fields, |ident| quote! { &self.#ident }),
        Plan::Enum(variants) => {
            if variants.is_empty() {
                quote! { match *self {} }
            } else {
                let arms = variants.iter().map(emit_display_variant_arm);
                quote! { match self { #(#arms)* } }
            }
        }
    };

    quote! {
        #[automatically_derived]
        impl #impl_generics ::core::fmt::Display for #name #ty_generics #synthesized_wc {
            fn fmt(&self, __f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                #alloc_import
                #body
            }
        }
    }
}
