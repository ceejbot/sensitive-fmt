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

#[test]
fn debug_generic_with_plain_field() {
    let h: Holder<u32> = Holder { id: 1, value: 99 };
    assert_eq!(format!("{:?}", h), "Holder { id: 1, value: 99 }");
}

#[test]
fn display_generic_with_plain_field() {
    let h: Holder<u32> = Holder { id: 1, value: 99 };
    assert_eq!(format!("{}", h), "Holder { id: 1, value: 99 }");
}

#[test]
fn redact_only_drops_bounds_on_t() {
    // Compiles ONLY if no `T: Debug` or `T: Display` bound is synthesized.
    // NotFormattable has neither impl.
    let r: RedactOnly<NotFormattable> = RedactOnly {
        id: 1,
        _value: NotFormattable,
    };
    assert_eq!(format!("{:?}", r), "RedactOnly { id: 1, _value: REDACTED }");
    assert_eq!(format!("{}", r), "RedactOnly { id: 1, _value: REDACTED }");
}

#[test]
fn lifetime_compiles_and_renders() {
    let w: WithLifetime<'_, u32> = WithLifetime {
        name: "alice",
        value: 7,
    };
    assert_eq!(format!("{:?}", w), r#"WithLifetime { name: "alice", value: 7 }"#);
    assert_eq!(format!("{}", w), "WithLifetime { name: alice, value: 7 }");
}
