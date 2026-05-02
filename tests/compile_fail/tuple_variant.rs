use sensitive_fmt::SensitiveDebug;

#[derive(SensitiveDebug)]
enum Bad {
    Good { id: u64 },
    Tupled(u64, String),
}

fn main() {}
