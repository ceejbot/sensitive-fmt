//! Plain Display: no attributes, single-line struct-name-prefixed output.

use sensitive_fmt::SensitiveDisplay;

#[derive(SensitiveDisplay)]
struct User {
    id: u64,
    name: String,
}

#[derive(SensitiveDisplay)]
struct Empty {}

#[test]
fn display_named_struct_uses_field_display() {
    let u = User {
        id: 42,
        name: "Alice".into(),
    };
    // Note: Display of `String` does NOT add quotes, so `name: Alice`
    // (contrast with Debug, which would be `name: "Alice"`).
    assert_eq!(format!("{u}"), "User { id: 42, name: Alice }");
}

#[test]
fn display_empty_struct_renders_with_braces() {
    // Spec: empty struct prints as `Foo {}`, matching the Debug derive.
    assert_eq!(format!("{}", Empty {}), "Empty {}");
}

#[derive(SensitiveDisplay)]
struct OneField {
    value: u32,
}

#[test]
fn display_single_field_no_separator() {
    let s = OneField { value: 7 };
    assert_eq!(format!("{s}"), "OneField { value: 7 }");
}
