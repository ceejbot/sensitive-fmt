//! Smoke test: the derive macros attach to a named-field struct without
//! errors. Does not exercise Debug or Display yet (that's Tasks 2 and 3).

use sensitive_fmt::{SensitiveDebug, SensitiveDisplay};

#[derive(SensitiveDebug, SensitiveDisplay)]
struct Smoke {
    id: u64,
    name: String,
}

#[test]
fn skeleton_compiles() {
    let s = Smoke {
        id: 1,
        name: "ok".into(),
    };
    let _ = s.id; // touch fields so dead-code lint is quiet
    let _ = s.name;
}
