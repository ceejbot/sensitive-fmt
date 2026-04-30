//! `#[sensitive(redact)]` field attribute.

use sensitive_fmt::{SensitiveDebug, SensitiveDisplay};

#[derive(SensitiveDebug, SensitiveDisplay)]
struct Account {
    id: u64,
    #[sensitive(redact)]
    _email: String,
    #[sensitive(redact)]
    _password: String,
}

/// A type with no Debug or Display impl. If `redact` truly drops the trait
/// requirement, this struct should still compile.
struct NotFormattable;

#[derive(SensitiveDebug, SensitiveDisplay)]
struct WithUnformattable {
    id: u64,
    #[sensitive(redact)]
    _raw: NotFormattable,
}

#[test]
fn redact_in_debug() {
    let a = Account {
        id: 1,
        _email: "alice@example.com".into(),
        _password: "hunter2".into(),
    };
    assert_eq!(
        format!("{:?}", a),
        "Account { id: 1, _email: REDACTED, _password: REDACTED }",
    );
}

#[test]
fn redact_in_display() {
    let a = Account {
        id: 1,
        _email: "alice@example.com".into(),
        _password: "hunter2".into(),
    };
    assert_eq!(
        format!("{}", a),
        "Account { id: 1, _email: REDACTED, _password: REDACTED }",
    );
}

#[test]
fn redact_drops_trait_bound() {
    let w = WithUnformattable {
        id: 7,
        _raw: NotFormattable,
    };
    assert_eq!(format!("{:?}", w), "WithUnformattable { id: 7, _raw: REDACTED }");
    assert_eq!(format!("{}", w), "WithUnformattable { id: 7, _raw: REDACTED }");
}
