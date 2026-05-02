//! Enum support: unit variants, named-field variants, and combinations.
//!
//! Tuple variants are rejected at compile time; that path is covered by
//! `tests/compile_fail/tuple_variant.rs`.

use sensitive_fmt::{SensitiveDebug, SensitiveDisplay};

#[derive(SensitiveDebug, SensitiveDisplay)]
enum Color {
    Red,
    Green,
    Blue,
}

#[derive(SensitiveDebug, SensitiveDisplay)]
enum PetRecord {
    Cat {
        breed: String,
        id: u64,
        #[sensitive(redact)]
        insurance_id: String,
        neutered: bool,
    },
    Goldfish {
        color: String,
        id: u64,
        #[sensitive(redact)]
        insurance_id: String,
    },
}

#[derive(SensitiveDebug, SensitiveDisplay)]
enum Mixed {
    Empty,
    EmptyBraced {},
    Loaded {
        id: u64,
        #[sensitive(redact)]
        secret: String,
    },
}

struct NotFormattable;

#[derive(SensitiveDebug, SensitiveDisplay)]
enum WithSkipped {
    Wrapped {
        id: u64,
        #[sensitive(skip)]
        _blob: NotFormattable,
    },
}

#[derive(SensitiveDebug, SensitiveDisplay)]
enum WithTruncate {
    Token {
        #[sensitive(truncate = 4)]
        token: String,
    },
}

#[test]
fn unit_variants_render_as_bare_name() {
    assert_eq!(format!("{:?}", Color::Red), "Red");
    assert_eq!(format!("{}", Color::Green), "Green");
    assert_eq!(format!("{:?}", Color::Blue), "Blue");
}

#[test]
fn struct_variant_redact_in_debug() {
    let cat = PetRecord::Cat {
        breed: "Tabby".into(),
        id: 1,
        insurance_id: "POL-123".into(),
        neutered: true,
    };
    assert_eq!(
        format!("{cat:?}"),
        r#"Cat { breed: "Tabby", id: 1, insurance_id: REDACTED, neutered: true }"#,
    );
}

#[test]
fn struct_variant_redact_in_display() {
    let fish = PetRecord::Goldfish {
        color: "Orange".into(),
        id: 2,
        insurance_id: "POL-789".into(),
    };
    assert_eq!(
        format!("{fish}"),
        "Goldfish { color: Orange, id: 2, insurance_id: REDACTED }",
    );
}

#[test]
fn second_variant_in_match_renders_correctly() {
    // Confirms each match arm wires up its own field bindings.
    let cat = PetRecord::Cat {
        breed: "Maine Coon".into(),
        id: 7,
        insurance_id: "POL-X".into(),
        neutered: false,
    };
    let fish = PetRecord::Goldfish {
        color: "Black".into(),
        id: 8,
        insurance_id: "POL-Y".into(),
    };
    assert_eq!(
        format!("{cat:?}"),
        r#"Cat { breed: "Maine Coon", id: 7, insurance_id: REDACTED, neutered: false }"#,
    );
    assert_eq!(
        format!("{fish:?}"),
        r#"Goldfish { color: "Black", id: 8, insurance_id: REDACTED }"#,
    );
}

#[test]
fn unit_variant_alongside_struct_variants() {
    assert_eq!(format!("{:?}", Mixed::Empty), "Empty");
    assert_eq!(format!("{}", Mixed::Empty), "Empty");
}

#[test]
fn empty_braced_variant_renders_with_braces() {
    assert_eq!(format!("{:?}", Mixed::EmptyBraced {}), "EmptyBraced {}");
    assert_eq!(format!("{}", Mixed::EmptyBraced {}), "EmptyBraced {}");
}

#[test]
fn loaded_variant_in_mixed_enum() {
    let m = Mixed::Loaded {
        id: 5,
        secret: "hunter2".into(),
    };
    assert_eq!(format!("{m:?}"), "Loaded { id: 5, secret: REDACTED }");
    assert_eq!(format!("{m}"), "Loaded { id: 5, secret: REDACTED }");
}

#[test]
fn skip_variant_drops_trait_bound() {
    // Compiles ONLY if no `Debug`/`Display` bound is synthesized for
    // the skipped field type. NotFormattable has neither impl.
    let w = WithSkipped::Wrapped {
        id: 3,
        _blob: NotFormattable,
    };
    assert_eq!(format!("{w:?}"), "Wrapped { id: 3, _blob: <skipped> }");
    assert_eq!(format!("{w}"), "Wrapped { id: 3, _blob: <skipped> }");
}

#[test]
fn truncate_in_struct_variant() {
    let t = WithTruncate::Token {
        token: "sk_live_abc123wxyz".into(),
    };
    assert_eq!(format!("{t:?}"), "Token { token: ****wxyz }");
    assert_eq!(format!("{t}"), "Token { token: ****wxyz }");
}

#[test]
fn debug_pretty_print_works_for_struct_variant() {
    let cat = PetRecord::Cat {
        breed: "Tabby".into(),
        id: 1,
        insurance_id: "POL-123".into(),
        neutered: true,
    };
    let pretty = format!("{cat:#?}");
    // Don't assert on exact indentation — that's a core::fmt detail.
    // Confirming the multi-line debug_struct path activated is enough.
    assert!(pretty.contains("Cat {"));
    assert!(pretty.contains("breed: \"Tabby\","));
    assert!(pretty.contains("insurance_id: REDACTED,"));
}
