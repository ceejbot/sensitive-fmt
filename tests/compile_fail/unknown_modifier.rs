use sensitive_fmt::SensitiveDebug;

#[derive(SensitiveDebug)]
struct Bad {
    #[sensitive(foo)]
    field: String,
}

fn main() {}
