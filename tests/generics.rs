//! Generic structs and bound synthesis.

use sensitive_fmt::{SensitiveDebug, SensitiveDisplay};

#[derive(SensitiveDebug, SensitiveDisplay)]
struct Holder<T> {
    id: u64,
    value: T,
}

/// `T` is only ever used in a redacted field — so no `T: Debug` / `T: Display`
/// bound should be required. If we accidentally over-bound, this won't compile
/// because `NotFormattable` doesn't implement Debug or Display.
struct NotFormattable;

#[derive(SensitiveDebug, SensitiveDisplay)]
struct RedactOnly<T> {
    id: u64,
    #[sensitive(redact)]
    _value: T,
}

#[derive(SensitiveDebug, SensitiveDisplay)]
struct WithLifetime<'a, T> {
    name: &'a str,
    value: T,
}

trait AssocTypes {
    type DebugValue;
    type DisplayValue;
    type Token;
    type Secret;
}

struct DemoTypes;

impl AssocTypes for DemoTypes {
    type DebugValue = Vec<u8>;
    type DisplayValue = &'static str;
    type Token = String;
    type Secret = NotFormattable;
}

#[derive(SensitiveDebug)]
#[allow(dead_code)]
struct AssocDebug<T>
where
    T: AssocTypes,
    <T as AssocTypes>::DebugValue: core::fmt::Debug,
{
    value: <T as AssocTypes>::DebugValue,
}

#[derive(SensitiveDisplay)]
struct AssocDisplay<T>
where
    T: AssocTypes,
    <T as AssocTypes>::DisplayValue: core::fmt::Display,
{
    value: <T as AssocTypes>::DisplayValue,
}

#[derive(SensitiveDebug, SensitiveDisplay)]
struct AssocTruncate<T>
where
    T: AssocTypes,
    <T as AssocTypes>::Token: core::fmt::Display,
{
    #[sensitive(truncate = 4)]
    token: <T as AssocTypes>::Token,
}

#[derive(SensitiveDebug, SensitiveDisplay)]
#[allow(dead_code)]
struct AssocHidden<T>
where
    T: AssocTypes,
{
    #[sensitive(redact)]
    redacted: <T as AssocTypes>::Secret,
    #[sensitive(skip)]
    skipped: <T as AssocTypes>::Secret,
}

struct Shown<T>(T);

impl<T: core::fmt::Debug> core::fmt::Debug for Shown<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(&self.0, f)
    }
}

impl<T: core::fmt::Display> core::fmt::Display for Shown<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Display::fmt(&self.0, f)
    }
}

#[derive(SensitiveDebug, SensitiveDisplay)]
struct WrapperWhere<T>
where
    Shown<T>: core::fmt::Debug + core::fmt::Display,
{
    value: Shown<T>,
}

// `PhantomData<T>` implements `Debug` for any `T` (the blanket impl does not
// require `T: Debug`). Field-type bound synthesis emits
// `PhantomData<T>: Debug`, which is always satisfied. A per-type-parameter
// strategy would over-bound by emitting `T: Debug`, making this fail to
// compile when `T = NotFormattable`.
#[derive(SensitiveDebug)]
#[allow(dead_code)]
struct PhantomHolder<T> {
    id: u64,
    marker: core::marker::PhantomData<T>,
}

#[test]
fn debug_generic_with_plain_field() {
    let h: Holder<u32> = Holder { id: 1, value: 99 };
    assert_eq!(format!("{h:?}"), "Holder { id: 1, value: 99 }");
}

#[test]
fn display_generic_with_plain_field() {
    let h: Holder<u32> = Holder { id: 1, value: 99 };
    assert_eq!(format!("{h}"), "Holder { id: 1, value: 99 }");
}

#[test]
fn redact_only_drops_bounds_on_t() {
    // Compiles ONLY if no `T: Debug` or `T: Display` bound is synthesized.
    // NotFormattable has neither impl.
    let r: RedactOnly<NotFormattable> = RedactOnly {
        id: 1,
        _value: NotFormattable,
    };
    assert_eq!(format!("{r:?}"), "RedactOnly { id: 1, _value: REDACTED }");
    assert_eq!(format!("{r}"), "RedactOnly { id: 1, _value: REDACTED }");
}

#[test]
fn lifetime_compiles_and_renders() {
    let w: WithLifetime<'_, u32> = WithLifetime {
        name: "alice",
        value: 7,
    };
    assert_eq!(format!("{w:?}"), r#"WithLifetime { name: "alice", value: 7 }"#);
    assert_eq!(format!("{w}"), "WithLifetime { name: alice, value: 7 }");
}

#[test]
fn associated_type_debug_field_compiles_and_renders() {
    let value = AssocDebug::<DemoTypes> { value: vec![1, 2, 3] };
    assert_eq!(format!("{value:?}"), "AssocDebug { value: [1, 2, 3] }");
}

#[test]
fn associated_type_display_field_compiles_and_renders() {
    let value = AssocDisplay::<DemoTypes> { value: "visible" };
    assert_eq!(format!("{value}"), "AssocDisplay { value: visible }");
}

#[test]
fn associated_type_truncate_field_uses_display() {
    let value = AssocTruncate::<DemoTypes> {
        token: "sk_live_abc123wxyz".into(),
    };
    assert_eq!(format!("{value:?}"), "AssocTruncate { token: ****wxyz }");
    assert_eq!(format!("{value}"), "AssocTruncate { token: ****wxyz }");
}

#[test]
fn associated_type_hidden_fields_do_not_need_formatting_bounds() {
    let value = AssocHidden::<DemoTypes> {
        redacted: NotFormattable,
        skipped: NotFormattable,
    };
    assert_eq!(
        format!("{value:?}"),
        "AssocHidden { redacted: REDACTED, skipped: <skipped> }",
    );
    assert_eq!(
        format!("{value}"),
        "AssocHidden { redacted: REDACTED, skipped: <skipped> }",
    );
}

#[test]
fn generic_wrapper_with_explicit_where_clause_compiles_and_renders() {
    let value = WrapperWhere { value: Shown(17) };
    assert_eq!(format!("{value:?}"), "WrapperWhere { value: 17 }");
    assert_eq!(format!("{value}"), "WrapperWhere { value: 17 }");
}

#[test]
fn phantom_data_field_does_not_overbound_type_param() {
    // `NotFormattable` has no `Debug` impl. This compiles only because the
    // synthesized bound is `PhantomData<NotFormattable>: Debug` (always true)
    // rather than `NotFormattable: Debug`.
    let h: PhantomHolder<NotFormattable> = PhantomHolder {
        id: 1,
        marker: core::marker::PhantomData,
    };
    assert_eq!(
        format!("{h:?}"),
        "PhantomHolder { id: 1, marker: PhantomData<generics::NotFormattable> }"
    );
}
