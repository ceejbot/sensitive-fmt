use sensitive_fmt::SensitiveDebug;

#[derive(SensitiveDebug)]
struct Bad {
    #[sensitive(truncate = -3)]
    field: String,
}

fn main() {}
