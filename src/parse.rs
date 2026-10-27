//! Turns a `DeriveInput` into the `Plan` that codegen consumes. Every
//! rejection is a `syn::Error` spanned to the offending token, so the user
//! sees it on the attribute or ident that caused it.

use proc_macro2::Ident;
use syn::{Attribute, Data, DataStruct, DeriveInput, Field, Fields, FieldsNamed, Meta};

/// What to emit for a single field, distilled from its `#[sensitive(...)]`
/// attribute.
pub enum FieldPlan {
    /// No attribute: format with the derive's own trait.
    Plain,
    Redact,
    Skip,
    /// The parser guarantees N >= 1.
    Truncate(u32),
}

/// One named field with its attribute already parsed. The type rides along
/// for bound synthesis.
pub struct PlannedField {
    pub ident: Ident,
    pub ty: syn::Type,
    pub plan: FieldPlan,
}

pub struct PlannedVariant {
    pub ident: Ident,
    pub shape: VariantShape,
}

/// Tuple variants never reach here; `plan_input` rejects them.
pub enum VariantShape {
    Unit,
    /// Possibly empty: `Variant {}` prints with braces, and codegen
    /// special-cases it.
    Struct(Vec<PlannedField>),
}

/// Every input shape codegen handles. Anything else fails in `plan_input`.
pub enum Plan {
    Struct(Vec<PlannedField>),
    Enum(Vec<PlannedVariant>),
}

/// Rejects tuple structs, unit structs, unions, tuple variants, and any
/// `#[sensitive(...)]` that is not on a field. Empty enums pass: codegen emits
/// an uninhabited `match *self {}` body.
pub fn plan_input(input: &DeriveInput, derive_name: &str) -> syn::Result<Plan> {
    reject_sensitive_attr(&input.attrs)?;
    match &input.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(n),
            ..
        }) => Ok(Plan::Struct(plan_named_fields(n)?)),
        Data::Struct(DataStruct {
            fields: Fields::Unnamed(_),
            ..
        })
        | Data::Struct(DataStruct {
            fields: Fields::Unit, ..
        }) => Err(syn::Error::new_spanned(
            &input.ident,
            format!("{derive_name} requires a struct with named fields"),
        )),
        Data::Enum(e) => {
            let mut variants = Vec::with_capacity(e.variants.len());
            for variant in &e.variants {
                reject_sensitive_attr(&variant.attrs)?;
                let shape = match &variant.fields {
                    Fields::Named(n) => VariantShape::Struct(plan_named_fields(n)?),
                    Fields::Unit => VariantShape::Unit,
                    Fields::Unnamed(_) => {
                        return Err(syn::Error::new_spanned(
                            &variant.ident,
                            format!(
                                "{derive_name} does not support tuple variants; \
                                 use named fields or wrap the payload in a struct"
                            ),
                        ));
                    }
                };
                variants.push(PlannedVariant {
                    ident: variant.ident.clone(),
                    shape,
                });
            }
            Ok(Plan::Enum(variants))
        }
        Data::Union(_) => Err(syn::Error::new_spanned(
            &input.ident,
            format!("{derive_name} does not support unions"),
        )),
    }
}

/// rustc accepts derive helper attributes anywhere inside the item, including
/// on the item itself and on enum variants. Only fields carry meaning here, so
/// a misplaced attribute must fail loudly rather than print the payload in
/// full.
fn reject_sensitive_attr(attrs: &[Attribute]) -> syn::Result<()> {
    match attrs.iter().find(|attr| attr.path().is_ident("sensitive")) {
        Some(attr) => Err(syn::Error::new_spanned(
            attr,
            "#[sensitive(...)] is only valid on fields",
        )),
        None => Ok(()),
    }
}

fn plan_named_fields(named: &FieldsNamed) -> syn::Result<Vec<PlannedField>> {
    let mut planned = Vec::with_capacity(named.named.len());
    for field in &named.named {
        let ident = field.ident.clone().expect("Fields::Named guarantees named fields");
        let plan = parse_field_plan(field)?;
        planned.push(PlannedField {
            ident,
            ty: field.ty.clone(),
            plan,
        });
    }
    Ok(planned)
}

fn parse_field_plan(field: &Field) -> syn::Result<FieldPlan> {
    let mut found: Option<(FieldPlan, &Attribute)> = None;
    for attr in &field.attrs {
        if !attr.path().is_ident("sensitive") {
            continue;
        }
        if found.is_some() {
            return Err(syn::Error::new_spanned(
                attr,
                "field already has a #[sensitive(...)] attribute",
            ));
        }
        let plan = parse_sensitive_attr(attr)?;
        found = Some((plan, attr));
    }
    Ok(found.map(|(p, _)| p).unwrap_or(FieldPlan::Plain))
}

fn parse_sensitive_attr(attr: &Attribute) -> syn::Result<FieldPlan> {
    // A full `Punctuated` parse, rather than `parse_nested_meta`, so that empty
    // `()` and multi-modifier `(a, b)` each get their own error.
    let metas: syn::punctuated::Punctuated<Meta, syn::Token![,]> =
        attr.parse_args_with(syn::punctuated::Punctuated::parse_terminated)?;

    if metas.is_empty() {
        return Err(syn::Error::new_spanned(
            attr,
            "expected one of: redact, truncate = N, skip",
        ));
    }
    if metas.len() > 1 {
        let names: Vec<String> = metas.iter().map(modifier_name).collect();
        return Err(syn::Error::new_spanned(
            attr,
            format!(
                "at most one sensitive modifier may be applied; found {} and {}",
                names[0], names[1],
            ),
        ));
    }

    let meta = metas.into_iter().next().unwrap();
    parse_one_modifier(meta)
}

fn parse_one_modifier(meta: Meta) -> syn::Result<FieldPlan> {
    match meta {
        Meta::Path(ref path) if path.is_ident("redact") => Ok(FieldPlan::Redact),
        Meta::Path(ref path) if path.is_ident("skip") => Ok(FieldPlan::Skip),
        Meta::NameValue(ref nv) if nv.path.is_ident("truncate") => {
            let n = parse_truncate_value(&nv.value, nv)?;
            Ok(FieldPlan::Truncate(n))
        }
        Meta::Path(path) => Err(syn::Error::new_spanned(
            &path,
            format!(
                r#"unknown sensitive modifier "{}"; expected one of: redact, truncate = N, skip"#,
                path_str(&path),
            ),
        )),
        Meta::NameValue(nv) => Err(syn::Error::new_spanned(
            &nv,
            format!(
                r#"unknown sensitive modifier "{} = ..."; expected one of: redact, truncate = N, skip"#,
                path_str(&nv.path),
            ),
        )),
        Meta::List(list) => Err(syn::Error::new_spanned(
            &list,
            "expected one of: redact, truncate = N, skip",
        )),
    }
}

fn modifier_name(m: &Meta) -> String {
    match m {
        Meta::Path(p) => path_str(p),
        Meta::NameValue(nv) => path_str(&nv.path),
        Meta::List(l) => path_str(&l.path),
    }
}

fn path_str(p: &syn::Path) -> String {
    p.segments
        .iter()
        .map(|s| s.ident.to_string())
        .collect::<Vec<_>>()
        .join("::")
}

fn parse_truncate_value(expr: &syn::Expr, nv: &syn::MetaNameValue) -> syn::Result<u32> {
    let lit = match expr {
        syn::Expr::Lit(syn::ExprLit { lit, .. }) => lit,
        // `-3` parses as `Expr::Unary(Neg, Lit(3))`, not `Expr::Lit`. Catch it
        // here so the error says "positive integer" rather than "integer
        // literal".
        syn::Expr::Unary(syn::ExprUnary {
            op: syn::UnOp::Neg(_), ..
        }) => {
            return Err(syn::Error::new_spanned(expr, "truncate must be a positive integer"));
        }
        _ => {
            return Err(syn::Error::new_spanned(nv, "truncate value must be an integer literal"));
        }
    };
    let int = match lit {
        syn::Lit::Int(i) => i,
        _ => {
            return Err(syn::Error::new_spanned(
                lit,
                "truncate value must be an integer literal",
            ));
        }
    };
    let n: u32 = int
        .base10_parse()
        .map_err(|_| syn::Error::new_spanned(int, "truncate must be a positive integer"))?;
    if n == 0 {
        return Err(syn::Error::new_spanned(int, "truncate must be at least 1"));
    }
    Ok(n)
}
