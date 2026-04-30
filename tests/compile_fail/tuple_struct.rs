use sensitive_fmt::SensitiveDebug;

#[derive(SensitiveDebug)]
struct Bad(u32, String);

fn main() {}
