//! `#[sensitive(skip)]` field attribute.

use sensitive_fmt::{SensitiveDebug, SensitiveDisplay};

struct NotFormattable;

#[derive(SensitiveDebug, SensitiveDisplay)]
struct Record {
    id: u64,
    #[sensitive(skip)]
    _raw: NotFormattable,
}

#[test]
fn skip_in_debug() {
    let r = Record {
        id: 1,
        _raw: NotFormattable,
    };
    assert_eq!(format!("{:?}", r), "Record { id: 1, _raw: <skipped> }");
}

#[test]
fn skip_in_display() {
    let r = Record {
        id: 1,
        _raw: NotFormattable,
    };
    assert_eq!(format!("{}", r), "Record { id: 1, _raw: <skipped> }");
}

/// A struct that uses both #[sensitive(redact)] and #[sensitive(skip)] on
/// different fields. Confirms the two modifiers coexist correctly — both
/// marker enum variants are in scope inside the same generated `fmt` body.
#[derive(SensitiveDebug, SensitiveDisplay)]
struct Mixed {
    id: u64,
    #[sensitive(redact)]
    _token: String,
    #[sensitive(skip)]
    _raw: NotFormattable,
}

#[test]
fn redact_and_skip_coexist_in_one_struct() {
    let m = Mixed {
        id: 1,
        _token: "secret".into(),
        _raw: NotFormattable,
    };
    assert_eq!(format!("{:?}", m), "Mixed { id: 1, _token: REDACTED, _raw: <skipped> }",);
    assert_eq!(format!("{}", m), "Mixed { id: 1, _token: REDACTED, _raw: <skipped> }",);
}
