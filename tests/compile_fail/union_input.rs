use sensitive_fmt::SensitiveDebug;

#[derive(SensitiveDebug)]
union Bad { a: u32, b: f32 }

fn main() {}
