use sensitive_fmt::SensitiveDebug;

#[derive(SensitiveDebug)]
struct Bad {
    #[sensitive()]
    field: String,
}

fn main() {}
