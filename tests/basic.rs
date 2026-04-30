//! Plain Debug: no attributes; mirrors std's #[derive(Debug)] for non-empty
//! structs, and prints "Foo {}" for empty structs (spec requirement).

use sensitive_fmt::SensitiveDebug;

#[derive(SensitiveDebug)]
struct User {
    _id: u64,
    _name: String,
}

#[derive(SensitiveDebug)]
struct Empty {}

#[derive(SensitiveDebug)]
#[allow(dead_code)]
struct Node {
    value: u32,
    next: Option<Box<Node>>,
}

#[test]
fn debug_named_struct() {
    let u = User {
        _id: 42,
        _name: "Alice".into(),
    };
    assert_eq!(format!("{:?}", u), r#"User { _id: 42, _name: "Alice" }"#);
}

#[test]
fn debug_empty_struct_renders_with_braces() {
    // Spec: empty struct prints as `Foo {}` (with braces), not std's `Foo`.
    // Codegen special-cases the zero-field case to match this.
    assert_eq!(format!("{:?}", Empty {}), "Empty {}");
}

#[test]
fn debug_pretty_print_works() {
    let u = User {
        _id: 42,
        _name: "Alice".into(),
    };
    let pretty = format!("{:#?}", u);
    // Don't assert on the exact indent — that's an internal detail of
    // core::fmt's pretty-printer. Verifying the substrings are present
    // (anywhere) is enough to confirm the debug_struct() path was taken.
    assert!(pretty.contains("User {"));
    assert!(pretty.contains("_id: 42,"));
    assert!(pretty.contains(r#"_name: "Alice","#));
}

#[test]
fn recursive_debug_struct_compiles_and_renders() {
    let node = Node { value: 1, next: None };
    assert_eq!(format!("{:?}", node), "Node { value: 1, next: None }");
}
