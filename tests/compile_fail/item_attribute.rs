use sensitive_fmt::{SensitiveDebug, SensitiveDisplay};

#[derive(SensitiveDebug)]
#[sensitive(redact)]
struct Credential {
    token: String,
}

#[derive(SensitiveDisplay)]
#[sensitive(skip)]
enum Event {
    Login { token: String },
}

fn main() {}
