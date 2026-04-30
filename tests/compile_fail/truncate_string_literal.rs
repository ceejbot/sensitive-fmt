use sensitive_fmt::SensitiveDebug;

#[derive(SensitiveDebug)]
struct Bad {
    #[sensitive(truncate = "4")]
    field: String,
}

fn main() {}
