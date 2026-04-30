use sensitive_fmt::SensitiveDebug;

#[derive(SensitiveDebug)]
struct Bad {
    #[sensitive(redact, skip)]
    field: String,
}

fn main() {}
