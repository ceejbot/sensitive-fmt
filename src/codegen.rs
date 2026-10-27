//! Code generation for the `SensitiveDebug` and `SensitiveDisplay` derives.

use std::collections::BTreeSet;

use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::ext::IdentExt;
use syn::{DeriveInput, Generics, Type, WhereClause};

use crate::parse::{FieldPlan, Plan, PlannedField, PlannedVariant, VariantShape};

/// Which derive is being generated. A `Plain` field needs `Debug` under one
/// and `Display` under the other; marked fields don't care.
#[derive(Clone, Copy)]
enum Derive {
    DebugDerive,
    DisplayDerive,
}

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
        // Truncate formats with `Display` before slicing, under both derives.
        (FieldPlan::Truncate(_), _) => Display,
    }
}

/// Records which of `type_params` the type `ty` mentions. Hand-rolled rather
/// than `syn::visit` so the crate doesn't enable syn's `visit` feature for one
/// walk. Type arguments under a segment named after the self type are skipped:
/// a `Vec<Wrapper<T>>` field is formatted by the impl being generated, so its
/// `T` needs no bound of its own.
fn collect_type_params(ty: &Type, type_params: &BTreeSet<String>, self_type: &Ident, found: &mut BTreeSet<String>) {
    match ty {
        Type::Path(tp) => {
            if let Some(qself) = &tp.qself {
                collect_type_params(&qself.ty, type_params, self_type, found);
            }

            // A type parameter appears either alone (`T`) or as the head of
            // an associated-type shorthand (`T::Value`); either way it is the
            // first segment of an unqualified path.
            if tp.qself.is_none()
                && tp.path.leading_colon.is_none()
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
        Type::FnPtr(f) => {
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
                || tp.path.segments.iter().any(|seg| {
                    seg.ident == *self_type
                        || seg.ident == "Self"
                        || path_arguments_contain_self(&seg.arguments, self_type)
                })
        }
        Type::Reference(r) => contains_self_type(&r.elem, self_type),
        Type::Array(a) => contains_self_type(&a.elem, self_type),
        Type::Slice(s) => contains_self_type(&s.elem, self_type),
        Type::Tuple(t) => t.elems.iter().any(|elem| contains_self_type(elem, self_type)),
        Type::Ptr(p) => contains_self_type(&p.elem, self_type),
        Type::Paren(p) => contains_self_type(&p.elem, self_type),
        Type::Group(g) => contains_self_type(&g.elem, self_type),
        Type::FnPtr(f) => {
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

/// Appends one predicate per formatted field to the user's `where` clause.
///
/// The bound goes on the field type itself (`Option<T>: Debug`), not on the
/// type parameters it mentions, the same strategy serde uses. Bounding the
/// field type is exact where a per-parameter `T: Debug` over-bounds:
/// `PhantomData<T>`, associated-type projections, and wrapper types with
/// their own conditional impls. Fields that mention no type parameter need no
/// predicate. Fields that mention the type being defined (`Box<Self>`,
/// `(T, Box<Self>)`) fall back to per-parameter bounds, since
/// `Wrapper<T>: Debug` on the impl for `Wrapper<T>` is self-referential.
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

/// Every field's (type, plan) pair, whether it lives in a struct or in an enum
/// variant. Bound synthesis doesn't care which, and duplicate predicates are
/// harmless.
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

/// Declares the local marker enum at the top of the generated `fmt` body.
///
/// `.field("name", &"REDACTED")` would print the quotes; the enum's `Debug`
/// impl writes the bare text instead. It is local to `fmt` so nothing leaks
/// into the user's namespace, and declared before any `match self` so every
/// arm can reach it. The `alloc` import and the `Truncated` variant appear
/// only when a field truncates, which keeps redact/skip-only types on `core`
/// alone.
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
                // Compare in u64 so a 16-bit `usize` cannot wrap `__n`; once
                // inside the branch `__n <= __count`, so it fits in usize.
                if (__count as u64) >= (*__n as u64) {
                    let __nm1 = (*__n as usize).saturating_sub(1);
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
        // Which variants get constructed depends on the user's attributes.
        #[allow(dead_code)]
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

/// One `__builder.field(...)` call. `access` is `&self.foo` for a struct and
/// the match-arm binding for an enum variant, which is already a reference.
fn debug_field_call(field: &PlannedField, access: &TokenStream) -> TokenStream {
    let label = field.ident.unraw().to_string();
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

/// One field's label and value for the Display body; the caller emits the
/// separators and braces. Truncation goes through the same marker enum the
/// Debug derive uses, so the generated code carries one copy of the
/// algorithm.
fn display_field_writes(field: &PlannedField, access: &TokenStream) -> TokenStream {
    let label = format!("{}: ", field.ident.unraw());
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
                ::core::fmt::Debug::fmt(
                    &__SensitiveFmtMarkers::Truncated(
                        __sensitive_fmt_alloc::format!("{}", #access),
                        #n_lit,
                    ),
                    __f,
                )?;
            }
        }
    };
    quote! {
        __f.write_str(#label)?;
        #value
    }
}

/// Body for one named-field group. `name_str` is what the output shows:
/// "Patient" for a struct, "Cat" for an enum variant. The empty case is
/// special-cased because `debug_struct(name).finish()` prints `Name` without
/// braces and the spec wants `Name {}`.
fn emit_debug_named_body(
    name_str: &str,
    fields: &[PlannedField],
    field_access: impl Fn(usize, &Ident) -> TokenStream,
) -> TokenStream {
    if fields.is_empty() {
        let empty_lit = format!("{name_str} {{}}");
        return quote! { __f.write_str(#empty_lit) };
    }
    let calls = fields.iter().enumerate().map(|(i, f)| {
        let access = field_access(i, &f.ident);
        debug_field_call(f, &access)
    });
    quote! {
        let mut __builder = __f.debug_struct(#name_str);
        #(#calls)*
        __builder.finish()
    }
}

/// Display flavor of `emit_debug_named_body`, written with `write_str`
/// directly.
fn emit_display_named_body(
    name_str: &str,
    fields: &[PlannedField],
    field_access: impl Fn(usize, &Ident) -> TokenStream,
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
        let access = field_access(i, &f.ident);
        writes.push(display_field_writes(f, &access));
    }
    writes.push(quote! { __f.write_str(" }") });
    quote! { #(#writes)* }
}

/// Local name the `index`th variant field is bound to inside a match arm.
/// Binding under the field's own name would let a field called `__f` or
/// `__builder` shadow the generated locals. Positional names never collide
/// with each other, and unlike a prefix on the user's ident they never
/// produce a mid-name `__` that trips `non_snake_case` in the user's crate.
fn variant_binding(index: usize) -> Ident {
    format_ident!("__field_{index}")
}

/// The `Self::Variant { a: __field_0, .. }` pattern shared by both derives.
fn variant_pattern(v_ident: &Ident, fields: &[PlannedField]) -> TokenStream {
    let bindings = fields.iter().enumerate().map(|(i, f)| {
        let ident = &f.ident;
        let bound = variant_binding(i);
        quote! { #ident: #bound }
    });
    quote! { Self::#v_ident { #(#bindings),* } }
}

fn emit_debug_variant_arm(variant: &PlannedVariant) -> TokenStream {
    let v_ident = &variant.ident;
    let v_str = v_ident.unraw().to_string();
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
            let pattern = variant_pattern(v_ident, fields);
            let body = emit_debug_named_body(&v_str, fields, |i, _| {
                let bound = variant_binding(i);
                quote! { #bound }
            });
            quote! {
                #pattern => { #body },
            }
        }
    }
}

fn emit_display_variant_arm(variant: &PlannedVariant) -> TokenStream {
    let v_ident = &variant.ident;
    let v_str = v_ident.unraw().to_string();
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
            let pattern = variant_pattern(v_ident, fields);
            let body = emit_display_named_body(&v_str, fields, |i, _| {
                let bound = variant_binding(i);
                quote! { #bound }
            });
            quote! {
                #pattern => { #body },
            }
        }
    }
}

/// Emit `impl Debug for #name`. Non-empty bodies go through
/// `f.debug_struct(...)`, so `{:#?}` pretty-printing works for free.
pub fn emit_debug_impl(input: &DeriveInput, plan: &Plan) -> TokenStream {
    let name = &input.ident;
    let (impl_generics, ty_generics, _inherited_wc) = input.generics.split_for_impl();
    let field_types = collect_field_types(plan);
    let synthesized_wc = synthesize_where(&input.generics, name, &field_types, Derive::DebugDerive);
    let name_str = name.unraw().to_string();

    let any_marked = field_types.iter().any(|(_, p)| !matches!(p, FieldPlan::Plain));
    let any_truncate = field_types.iter().any(|(_, p)| matches!(p, FieldPlan::Truncate(_)));
    let marker_decl = build_marker_decl(any_marked, any_truncate);

    let body = match plan {
        Plan::Struct(fields) => emit_debug_named_body(&name_str, fields, |_, ident| quote! { &self.#ident }),
        Plan::Enum(variants) => {
            if variants.is_empty() {
                // `*self`, not `self`: a reference to an uninhabited type is
                // itself inhabited, so `match self {}` is non-exhaustive.
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

/// Emit `impl Display for #name`. Redact and skip write their literals
/// directly; only truncate needs the marker enum, so it is declared only when
/// a field truncates.
pub fn emit_display_impl(input: &DeriveInput, plan: &Plan) -> TokenStream {
    let name = &input.ident;
    let (impl_generics, ty_generics, _inherited_wc) = input.generics.split_for_impl();
    let field_types = collect_field_types(plan);
    let synthesized_wc = synthesize_where(&input.generics, name, &field_types, Derive::DisplayDerive);
    let name_str = name.unraw().to_string();

    let any_truncate = field_types.iter().any(|(_, p)| matches!(p, FieldPlan::Truncate(_)));
    let marker_decl = build_marker_decl(any_truncate, any_truncate);

    let body = match plan {
        Plan::Struct(fields) => emit_display_named_body(&name_str, fields, |_, ident| quote! { &self.#ident }),
        Plan::Enum(variants) => {
            if variants.is_empty() {
                // `*self`, not `self`: see the Debug derive.
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
                #marker_decl
                #body
            }
        }
    }
}
