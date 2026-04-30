use sensitive_fmt::SensitiveDebug;

#[derive(SensitiveDebug)]
struct Bad {
    #[sensitive(truncate = 0)]
    field: String,
}

fn main() {}
