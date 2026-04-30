use sensitive_fmt::SensitiveDebug;

#[derive(SensitiveDebug)]
struct Bad {
    #[sensitive(redact)]
    #[sensitive(skip)]
    field: String,
}

fn main() {}
