//! `#[sensitive(truncate = N)]` field attribute.

use sensitive_fmt::{SensitiveDebug, SensitiveDisplay};

#[derive(SensitiveDebug, SensitiveDisplay)]
struct Token {
    id: u64,
    #[sensitive(truncate = 4)]
    secret: String,
}

#[derive(SensitiveDebug, SensitiveDisplay)]
struct Mixed {
    id: u64,
    #[sensitive(truncate = 4)]
    code_point_value: String,
}

#[test]
fn truncate_long_value_keeps_last_n() {
    let t = Token {
        id: 1,
        secret: "sk_live_abc123wxyz".into(),
    };
    assert_eq!(format!("{:?}", t), "Token { id: 1, secret: ****wxyz }");
    assert_eq!(format!("{}", t), "Token { id: 1, secret: ****wxyz }");
}

#[test]
fn truncate_exactly_n_chars_passes() {
    let t = Token {
        id: 1,
        secret: "abcd".into(),
    };
    assert_eq!(format!("{:?}", t), "Token { id: 1, secret: ****abcd }");
    assert_eq!(format!("{}", t), "Token { id: 1, secret: ****abcd }");
}

#[test]
fn truncate_short_value_fully_redacts() {
    let t = Token {
        id: 1,
        secret: "abc".into(),
    }; // 3 chars, N = 4
    assert_eq!(format!("{:?}", t), "Token { id: 1, secret: REDACTED }");
    assert_eq!(format!("{}", t), "Token { id: 1, secret: REDACTED }");
}

#[test]
fn truncate_empty_value_fully_redacts() {
    let t = Token {
        id: 1,
        secret: String::new(),
    };
    assert_eq!(format!("{:?}", t), "Token { id: 1, secret: REDACTED }");
    assert_eq!(format!("{}", t), "Token { id: 1, secret: REDACTED }");
}

#[test]
fn truncate_counts_code_points_not_bytes() {
    // "fé🔑x" is 4 code points (the last 4 of "café🔑x").
    // Bytes vary: 'f'=1, 'é'=2, '🔑'=4, 'x'=1.
    let m = Mixed {
        id: 1,
        code_point_value: "café🔑x".into(),
    };
    assert_eq!(format!("{:?}", m), "Mixed { id: 1, code_point_value: ****fé🔑x }");
    assert_eq!(format!("{}", m), "Mixed { id: 1, code_point_value: ****fé🔑x }");
}
