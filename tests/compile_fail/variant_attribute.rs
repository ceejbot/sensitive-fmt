use sensitive_fmt::SensitiveDebug;

#[derive(SensitiveDebug)]
enum Event {
    #[sensitive(redact)]
    Login { token: String },
}

fn main() {}
